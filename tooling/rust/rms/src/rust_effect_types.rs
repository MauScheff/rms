//! Bounded source-declared receiver types. Unknown and ambiguous types stay unknown.
use std::collections::{BTreeMap, BTreeSet};
use syn::visit::{self, Visit};
use syn::{Expr, FnArg, GenericArgument, Item, Pat, PathArguments, ReturnType, Signature, Type};

fn use_tree_has_glob(tree: &syn::UseTree) -> bool {
    match tree {
        syn::UseTree::Glob(_) => true,
        syn::UseTree::Path(path) => use_tree_has_glob(&path.tree),
        syn::UseTree::Group(group) => group.items.iter().any(use_tree_has_glob),
        _ => false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RustValueType {
    Unknown,
    Named(String),
    Sequence(Box<Self>),
    Iterator(Box<Self>),
    Optional(Box<Self>),
    MapValues(Box<Self>),
    ResultOk(Box<Self>),
    Tuple(Vec<Self>),
}

impl RustValueType {
    pub(super) fn name(&self) -> Option<&String> {
        match self {
            Self::Named(name) => Some(name),
            _ => None,
        }
    }
    pub(super) fn element(&self) -> Option<&Self> {
        match self {
            Self::Iterator(element) | Self::Optional(element) => Some(element),
            _ => None,
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct RustTypeIndex {
    unique_types: BTreeSet<String>,
    shadowed_types: BTreeSet<String>,
    returns: BTreeMap<String, RustValueType>,
    fields: BTreeMap<String, RustValueType>,
    function_returns: BTreeMap<String, RustValueType>,
    function_parameters: BTreeMap<String, Vec<RustValueType>>,
    sources: BTreeMap<String, String>,
    path: String,
}

impl RustTypeIndex {
    pub(super) fn from_sources(sources: &BTreeMap<String, String>) -> Self {
        let files = sources
            .iter()
            .filter(|(path, _)| path.ends_with(".rs"))
            .filter_map(|(path, source)| syn::parse_file(source).ok().map(|file| (path, file)))
            .collect::<Vec<_>>();
        let mut counts = BTreeMap::<String, usize>::new();
        for (_, file) in &files {
            for item in &file.items {
                let name = match item {
                    Item::Struct(item) => Some(&item.ident),
                    Item::Enum(item) => Some(&item.ident),
                    Item::Type(item) => Some(&item.ident),
                    _ => None,
                };
                if let Some(name) = name {
                    *counts.entry(name.to_string()).or_default() += 1;
                }
            }
        }
        let mut index = Self {
            unique_types: counts
                .iter()
                .filter(|(_, count)| **count == 1)
                .map(|(name, _)| name.clone())
                .collect(),
            shadowed_types: counts.keys().cloned().collect(),
            returns: BTreeMap::new(),
            fields: BTreeMap::new(),
            function_returns: BTreeMap::new(),
            function_parameters: BTreeMap::new(),
            sources: sources.clone(),
            path: String::new(),
        };
        let mut signatures = BTreeMap::<String, Vec<(String, Type)>>::new();
        for (path, file) in &files {
            for item in &file.items {
                if let Item::Struct(item) = item {
                    if item.generics.params.is_empty()
                        && index.unique_types.contains(&item.ident.to_string())
                    {
                        for (ordinal, field) in item.fields.iter().enumerate() {
                            if let Some(value) = index.for_path(path).parse_type(&field.ty) {
                                let field_name = field
                                    .ident
                                    .as_ref()
                                    .map(ToString::to_string)
                                    .unwrap_or_else(|| ordinal.to_string());
                                index
                                    .fields
                                    .insert(format!("{}::{field_name}", item.ident), value);
                            }
                        }
                    }
                }
                if let Item::Fn(function) = item {
                    if function.sig.generics.params.is_empty() && function.attrs.is_empty() {
                        let parameters = function
                            .sig
                            .inputs
                            .iter()
                            .map(|argument| match argument {
                                FnArg::Typed(argument) => index
                                    .for_path(path)
                                    .parse_type(&argument.ty)
                                    .unwrap_or(RustValueType::Unknown),
                                _ => RustValueType::Unknown,
                            })
                            .collect();
                        index
                            .function_parameters
                            .insert(format!("{path}#{}", function.sig.ident), parameters);
                        if let ReturnType::Type(_, result) = &function.sig.output {
                            if let Some(value) = index.for_path(path).parse_type(result) {
                                index
                                    .function_returns
                                    .insert(format!("{path}#{}", function.sig.ident), value);
                            }
                        }
                    }
                }
                let Item::Impl(item) = item else { continue };
                if item.trait_.is_some() || !item.generics.params.is_empty() {
                    continue;
                }
                let Type::Path(owner) = item.self_ty.as_ref() else {
                    continue;
                };
                let Some(owner) = owner.path.get_ident() else {
                    continue;
                };
                if !index.unique_types.contains(&owner.to_string()) {
                    continue;
                }
                for member in &item.items {
                    let syn::ImplItem::Fn(function) = member else {
                        continue;
                    };
                    if !function.sig.generics.params.is_empty() {
                        continue;
                    }
                    let key = format!("{owner}::{}", function.sig.ident);
                    let result = match &function.sig.output {
                        ReturnType::Type(_, value) if matches!(value.as_ref(), Type::Path(path) if path.path.is_ident("Self")) => {
                            *item.self_ty.clone()
                        }
                        ReturnType::Type(_, value) => *value.clone(),
                        ReturnType::Default => Type::Verbatim(Default::default()),
                    };
                    signatures
                        .entry(key)
                        .or_default()
                        .push(((*path).clone(), result));
                }
            }
        }
        for (key, values) in signatures {
            if values.len() == 1 {
                if let Some(value) = index.for_path(&values[0].0).parse_type(&values[0].1) {
                    index.returns.insert(key, value);
                }
            }
        }
        index
    }

    pub(super) fn for_path(&self, path: &str) -> Self {
        let mut result = self.clone();
        result.path = path.to_string();
        result
    }

    pub(super) fn local_type_hints(
        &self,
        function: &syn::ItemFn,
    ) -> BTreeMap<String, RustValueType> {
        #[derive(Default)]
        struct Uses<'a> {
            counts: BTreeMap<String, usize>,
            vectors: BTreeSet<String>,
            calls: Vec<&'a syn::ExprCall>,
        }
        impl<'ast> Visit<'ast> for Uses<'ast> {
            fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
                *self.counts.entry(pattern.ident.to_string()).or_default() += 1;
                visit::visit_pat_ident(self, pattern);
            }
            fn visit_local(&mut self, local: &'ast syn::Local) {
                if let (Pat::Ident(name), Some(init)) = (&local.pat, &local.init) {
                    if matches!(init.expr.as_ref(), Expr::Call(call) if matches!(call.func.as_ref(), Expr::Path(path) if path.path.segments.len() == 2 && path.path.segments[0].ident == "Vec" && path.path.segments[1].ident == "new"))
                    {
                        self.vectors.insert(name.ident.to_string());
                    }
                }
                visit::visit_local(self, local);
            }
            fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
                self.calls.push(call);
                visit::visit_expr_call(self, call);
            }
            fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
                *self
                    .counts
                    .entry(function.sig.ident.to_string())
                    .or_default() += 1;
                visit::visit_item_fn(self, function);
            }
            fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
                let mut aliases = BTreeMap::new();
                super::collect_rust_use_aliases(&item.tree, Vec::new(), &mut aliases);
                for name in aliases.into_keys() {
                    *self.counts.entry(name).or_default() += 1;
                }
                if use_tree_has_glob(&item.tree) {
                    self.counts.insert("*".into(), 1);
                }
            }
            fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) { *self.counts.entry(item.ident.to_string()).or_default() += 1; }
            fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) { *self.counts.entry(item.ident.to_string()).or_default() += 1; }
            fn visit_item_type(&mut self, item: &'ast syn::ItemType) { *self.counts.entry(item.ident.to_string()).or_default() += 1; }
            fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) { *self.counts.entry(item.ident.to_string()).or_default() += 1; visit::visit_item_mod(self, item); }
        }
        let mut uses = Uses::default();
        uses.visit_signature(&function.sig);
        uses.visit_block(&function.block);
        if self.shadowed_types.contains("Vec") || uses.counts.contains_key("*") || uses.counts.contains_key("Vec") {
            return BTreeMap::new();
        }
        let Some(source) = self.sources.get(&self.path) else {
            return BTreeMap::new();
        };
        let Ok(file) = syn::parse_file(source) else {
            return BTreeMap::new();
        };
        if file
            .items
            .iter()
            .any(|item| matches!(item, Item::Use(item) if use_tree_has_glob(&item.tree)))
        {
            return BTreeMap::new();
        }
        if super::rust_import_aliases(&file).contains_key("Vec") {
            return BTreeMap::new();
        }
        let mut constraints = BTreeMap::<String, Vec<RustValueType>>::new();
        for call in uses.calls {
            let Expr::Path(path) = call.func.as_ref() else {
                continue;
            };
            let Some(callee) = path.path.get_ident() else {
                continue;
            };
            if uses.counts.contains_key(&callee.to_string()) {
                continue;
            }
            let Some(exact) = super::rust_exact_function_reference(
                &self.path,
                &callee.to_string(),
                &self.sources,
                0,
            ) else {
                continue;
            };
            let Some(parameters) = self.function_parameters.get(&exact) else {
                continue;
            };
            if parameters.len() != call.args.len() {
                continue;
            }
            for (argument, expected) in call.args.iter().zip(parameters) {
                let Expr::Reference(reference) = argument else {
                    continue;
                };
                let Expr::Path(variable) = reference.expr.as_ref() else {
                    continue;
                };
                let Some(name) = variable.path.get_ident().map(ToString::to_string) else {
                    continue;
                };
                if reference.mutability.is_some()
                    && uses.vectors.contains(&name)
                    && uses.counts.get(&name) == Some(&1)
                    && matches!(expected, RustValueType::Sequence(_))
                {
                    constraints.entry(name).or_default().push(expected.clone());
                }
            }
        }
        constraints
            .into_iter()
            .filter_map(|(name, values)| {
                values
                    .first()
                    .filter(|first| values.iter().all(|value| value == *first))
                    .map(|value| (name, value.clone()))
            })
            .collect()
    }

    fn parse_type(&self, value: &Type) -> Option<RustValueType> {
        match value {
            Type::Tuple(tuple) => Some(RustValueType::Tuple(
                tuple
                    .elems
                    .iter()
                    .map(|value| self.parse_type(value).unwrap_or(RustValueType::Unknown))
                    .collect(),
            )),
            Type::Reference(reference) => self.parse_type(&reference.elem),
            Type::Slice(slice) => self
                .parse_type(&slice.elem)
                .map(|element| RustValueType::Sequence(Box::new(element))),
            Type::Path(path) if path.qself.is_none() => {
                let segment = path.path.segments.last()?;
                let name = segment.ident.to_string();
                if matches!(segment.arguments, PathArguments::None)
                    && self.unique_types.contains(&name)
                {
                    let reference = path
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    super::rust_exact_item_reference(&self.path, &reference, &self.sources, 0)?;
                    return Some(RustValueType::Named(name));
                }
                if self.shadowed_types.contains(&name) {
                    return None;
                }
                if path.path.segments.len() == 1
                    && matches!(
                        name.as_str(),
                        "u8" | "u16"
                            | "u32"
                            | "u64"
                            | "usize"
                            | "i8"
                            | "i16"
                            | "i32"
                            | "i64"
                            | "isize"
                            | "bool"
                            | "str"
                    )
                {
                    return Some(RustValueType::Named(name));
                }
                let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                    return None;
                };
                let expected_arguments = if matches!(name.as_str(), "Result" | "BTreeMap") {
                    2
                } else {
                    1
                };
                if arguments.args.len() != expected_arguments || path.path.segments.len() != 1 {
                    return None;
                }
                let file = syn::parse_file(self.sources.get(&self.path)?).ok()?;
                fn has_glob(tree: &syn::UseTree) -> bool {
                    match tree {
                        syn::UseTree::Glob(_) => true,
                        syn::UseTree::Path(path) => has_glob(&path.tree),
                        syn::UseTree::Group(group) => group.items.iter().any(has_glob),
                        _ => false,
                    }
                }
                if file
                    .items
                    .iter()
                    .any(|item| matches!(item, Item::Use(item) if has_glob(&item.tree)))
                {
                    return None;
                }
                let aliases = super::rust_import_aliases(&file);
                if aliases.get(&name).is_some_and(|target| {
                    !matches!(
                        target.as_str(),
                        "std::collections::BTreeMap"
                            | "std::vec::Vec"
                            | "std::option::Option"
                            | "std::result::Result"
                    )
                }) {
                    return None;
                }
                if name == "BTreeMap"
                    && aliases.get(&name).map(String::as_str) != Some("std::collections::BTreeMap")
                {
                    return None;
                }
                let element_index = if name == "BTreeMap" { 1 } else { 0 };
                let GenericArgument::Type(element) = &arguments.args[element_index] else {
                    return None;
                };
                let element = Box::new(self.parse_type(element)?);
                match name.as_str() {
                    "Vec" => Some(RustValueType::Sequence(element)),
                    "Option" => Some(RustValueType::Optional(element)),
                    "BTreeMap" => Some(RustValueType::MapValues(element)),
                    "Result" => Some(RustValueType::ResultOk(element)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub(super) fn parameters(&self, signature: &Signature) -> BTreeMap<String, RustValueType> {
        let generic_names = signature
            .generics
            .type_params()
            .map(|parameter| parameter.ident.to_string())
            .collect::<BTreeSet<_>>();
        signature
            .inputs
            .iter()
            .filter_map(|argument| {
                let FnArg::Typed(argument) = argument else {
                    return None;
                };
                let Pat::Ident(name) = argument.pat.as_ref() else {
                    return None;
                };
                let value = self
                    .parse_type(&argument.ty)
                    .unwrap_or(RustValueType::Unknown);
                if value
                    .name()
                    .is_some_and(|name| generic_names.contains(name))
                {
                    return None;
                }
                Some((name.ident.to_string(), value))
            })
            .collect()
    }

    pub(super) fn pattern_bindings(
        &self,
        pattern: &Pat,
        value: &RustValueType,
    ) -> BTreeMap<String, RustValueType> {
        match (pattern, value) {
            (Pat::Ident(name), value) if name.subpat.is_none() => {
                BTreeMap::from([(name.ident.to_string(), value.clone())])
            }
            (Pat::Type(typed), value) => self.pattern_bindings(
                &typed.pat,
                &self.parse_type(&typed.ty).unwrap_or_else(|| value.clone()),
            ),
            (Pat::Reference(reference), value) => self.pattern_bindings(&reference.pat, value),
            (Pat::Tuple(pattern), RustValueType::Tuple(items))
                if pattern.elems.len() == items.len() =>
            {
                pattern
                    .elems
                    .iter()
                    .zip(items)
                    .flat_map(|(pattern, value)| self.pattern_bindings(pattern, value))
                    .collect()
            }
            (Pat::TupleStruct(pattern), RustValueType::Optional(element))
                if pattern
                    .path
                    .segments
                    .last()
                    .is_some_and(|name| name.ident == "Some")
                    && pattern.elems.len() == 1 =>
            {
                self.pattern_bindings(&pattern.elems[0], element)
            }
            (Pat::Slice(pattern), RustValueType::Sequence(element)) => pattern
                .elems
                .iter()
                .flat_map(|pattern| self.pattern_bindings(pattern, element))
                .collect(),
            _ => BTreeMap::new(),
        }
    }

    pub(super) fn expression_type(
        &self,
        expression: &Expr,
        values: &BTreeMap<String, RustValueType>,
    ) -> Option<RustValueType> {
        match expression {
            Expr::Tuple(tuple) => Some(RustValueType::Tuple(
                tuple
                    .elems
                    .iter()
                    .map(|value| {
                        self.expression_type(value, values)
                            .unwrap_or(RustValueType::Unknown)
                    })
                    .collect(),
            )),
            Expr::Field(field) => match self.expression_type(&field.base, values)? {
                RustValueType::Named(owner) => {
                    let name = match &field.member {
                        syn::Member::Named(name) => name.to_string(),
                        syn::Member::Unnamed(index) => index.index.to_string(),
                    };
                    self.fields.get(&format!("{owner}::{name}")).cloned()
                }
                RustValueType::Tuple(items) => match &field.member {
                    syn::Member::Unnamed(index) => items.get(index.index as usize).cloned(),
                    _ => None,
                },
                _ => None,
            },
            Expr::Path(path) => values.get(&path.path.get_ident()?.to_string()).cloned(),
            Expr::Reference(reference) => self.expression_type(&reference.expr, values),
            Expr::Paren(paren) => self.expression_type(&paren.expr, values),
            Expr::Try(value) => match self.expression_type(&value.expr, values)? {
                RustValueType::ResultOk(value) => Some(*value),
                _ => None,
            },
            Expr::Call(call) => {
                let Expr::Path(path) = call.func.as_ref() else {
                    return None;
                };
                if values.contains_key(&path.path.segments.first()?.ident.to_string()) {
                    return None;
                }
                let reference = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let exact =
                    super::rust_exact_function_reference(&self.path, &reference, &self.sources, 0)?;
                self.function_returns
                    .get(&exact)
                    .or_else(|| self.returns.get(exact.split_once('#')?.1))
                    .cloned()
            }
            Expr::MethodCall(call) => {
                let receiver = self.expression_type(&call.receiver, values)?;
                let is_optional = matches!(&receiver, RustValueType::Optional(_));
                let method = call.method.to_string();
                match receiver {
                    RustValueType::Named(name) => {
                        self.returns.get(&format!("{name}::{method}")).cloned()
                    }
                    RustValueType::Sequence(element) => match method.as_str() {
                        "iter" | "into_iter" => Some(RustValueType::Iterator(element)),
                        "as_slice" => Some(RustValueType::Sequence(element)),
                        "first" | "last" => Some(RustValueType::Optional(element)),
                        _ => None,
                    },
                    RustValueType::MapValues(element)
                        if matches!(method.as_str(), "values" | "into_values") =>
                    {
                        Some(RustValueType::Iterator(element))
                    }
                    RustValueType::Iterator(element)
                        if matches!(
                            method.as_str(),
                            "filter" | "take" | "skip" | "copied" | "cloned"
                        ) =>
                    {
                        Some(RustValueType::Iterator(element))
                    }
                    RustValueType::Iterator(element)
                        if matches!(
                            method.as_str(),
                            "find"
                                | "min"
                                | "max"
                                | "min_by"
                                | "max_by"
                                | "min_by_key"
                                | "max_by_key"
                        ) =>
                    {
                        Some(RustValueType::Optional(element))
                    }
                    RustValueType::Optional(element)
                        if matches!(
                            method.as_str(),
                            "filter" | "or_else" | "as_ref" | "copied" | "cloned"
                        ) =>
                    {
                        Some(RustValueType::Optional(element))
                    }
                    RustValueType::Optional(element)
                        if matches!(method.as_str(), "unwrap" | "unwrap_or" | "unwrap_or_else") =>
                    {
                        Some(*element)
                    }
                    RustValueType::Iterator(element) | RustValueType::Optional(element)
                        if matches!(method.as_str(), "map" | "and_then" | "filter_map") =>
                    {
                        let Expr::Closure(closure) = call.args.first()? else {
                            return None;
                        };
                        if closure.inputs.len() != 1 {
                            return None;
                        }
                        let Pat::Ident(parameter) = &closure.inputs[0] else {
                            return None;
                        };
                        let mut values = values.clone();
                        values.insert(parameter.ident.to_string(), *element);
                        let result = self.expression_type(&closure.body, &values)?;
                        if method == "and_then" {
                            Some(result)
                        } else if method == "filter_map" {
                            match result {
                                RustValueType::Optional(value) => {
                                    Some(RustValueType::Iterator(value))
                                }
                                _ => None,
                            }
                        } else if is_optional {
                            Some(RustValueType::Optional(Box::new(result)))
                        } else {
                            Some(RustValueType::Iterator(Box::new(result)))
                        }
                    }
                    RustValueType::Iterator(element)
                        if method == "collect"
                            && call.turbofish.as_ref().is_some_and(|arguments| {
                                if arguments.args.len() != 1 {
                                    return false;
                                }
                                let GenericArgument::Type(Type::Path(target)) = &arguments.args[0]
                                else {
                                    return false;
                                };
                                if target.path.segments.len() != 1
                                    || target.path.segments[0].ident != "Vec"
                                    || self.shadowed_types.contains("Vec")
                                {
                                    return false;
                                }
                                // Require an explicit standard Vec target; an unconstrained
                                // FromIterator implementation may produce a different type.
                                let Some(source) = self.sources.get(&self.path) else {
                                    return false;
                                };
                                let Ok(file) = syn::parse_file(source) else {
                                    return false;
                                };
                                super::rust_import_aliases(&file).get("Vec").is_none()
                            }) =>
                    {
                        Some(RustValueType::Sequence(element))
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

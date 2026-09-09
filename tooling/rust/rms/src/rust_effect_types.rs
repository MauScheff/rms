//! Bounded source-declared receiver types. Unknown and ambiguous types stay unknown.
use std::collections::{BTreeMap, BTreeSet};
use syn::{Expr, FnArg, GenericArgument, Item, Pat, PathArguments, ReturnType, Signature, Type};

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
    function_returns: BTreeMap<String, RustValueType>,
    fields: BTreeMap<(String, String), RustValueType>,
    mutable_sequences: BTreeMap<String, Vec<Option<RustValueType>>>,
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
            function_returns: BTreeMap::new(),
            fields: BTreeMap::new(),
            mutable_sequences: BTreeMap::new(),
            sources: sources.clone(),
            path: String::new(),
        };
        let mut signatures = BTreeMap::<String, Vec<(String, Type)>>::new();
        for (path, file) in &files {
            for item in &file.items {
                if let Item::Struct(item) = item {
                    if index.unique_types.contains(&item.ident.to_string()) && item.generics.params.is_empty()
                        && !item.attrs.iter().any(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr")) {
                        for (position, field) in item.fields.iter().enumerate() {
                            if !field.attrs.is_empty() { continue; }
                            if let Some(value) = index.for_path(path).parse_type(&field.ty) {
                                let name = field.ident.as_ref().map(ToString::to_string).unwrap_or_else(|| position.to_string());
                                index.fields.insert((item.ident.to_string(), name), value);
                            }
                        }
                    }
                }
                if let Item::Fn(function) = item {
                    if function.sig.generics.params.iter().all(|p| matches!(p, syn::GenericParam::Lifetime(_))) && function.attrs.is_empty() {
                        let parameters = function.sig.inputs.iter().map(|argument| {
                            let FnArg::Typed(argument) = argument else { return None; };
                            let Type::Reference(reference) = argument.ty.as_ref() else { return None; };
                            if reference.mutability.is_none() { return None; }
                            index.for_path(path).parse_type(&reference.elem).filter(|value| matches!(value, RustValueType::Sequence(_)))
                        }).collect();
                        index.mutable_sequences.insert(format!("{path}#{}", function.sig.ident), parameters);
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

    fn parse_type(&self, value: &Type) -> Option<RustValueType> {
        match value {
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

    pub(super) fn pattern_bindings(&self, pattern: &Pat, value: &RustValueType) -> BTreeMap<String, RustValueType> {
        let mut bindings = BTreeMap::new();
        match (pattern, value) {
            (Pat::Ident(name), value) => { bindings.insert(name.ident.to_string(), value.clone()); }
            (Pat::Reference(pattern), value) => { return self.pattern_bindings(&pattern.pat, value); }
            (Pat::TupleStruct(pattern), RustValueType::Optional(element))
                if pattern.path.is_ident("Some") && pattern.elems.len() == 1 => {
                return self.pattern_bindings(&pattern.elems[0], element);
            }
            (Pat::Tuple(pattern), RustValueType::Tuple(values)) if pattern.elems.len() == values.len() => {
                for (pattern, value) in pattern.elems.iter().zip(values) { bindings.extend(self.pattern_bindings(pattern, value)); }
            }
            _ => {}
        }
        bindings
    }

    // Rust fixes a local's type across all branches. An exact non-generic
    // function's `&mut Vec<T>` parameter constrains an empty Vec local without
    // inferring anything about callback behavior or arbitrary constructors.
    pub(super) fn empty_sequence_constraint(&self, name: &str, block: &syn::Block, scope: &BTreeMap<String, RustValueType>) -> Option<RustValueType> {
        use syn::visit::{self, Visit};
        struct Constraints<'a> {
            index: &'a RustTypeIndex,
            name: &'a str,
            values: Vec<RustValueType>,
            rebound: bool,
            declarations: usize,
            bound: BTreeSet<String>,
            callees: BTreeSet<String>,
        }
        impl<'ast> Visit<'ast> for Constraints<'_> {
            fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
                if node.ident == self.name { self.declarations += 1; }
                self.bound.insert(node.ident.to_string());
                visit::visit_pat_ident(self, node);
            }
            fn visit_item(&mut self, _: &'ast Item) { self.rebound = true; }
            fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
                if let Expr::Path(path) = call.func.as_ref() {
                    if let Some(root) = path.path.segments.first() { self.callees.insert(root.ident.to_string()); }
                    let reference = path.path.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>().join("::");
                    if let Some(exact) = super::rust_exact_function_reference(&self.index.path, &reference, &self.index.sources, 0) {
                        if let Some(parameters) = self.index.mutable_sequences.get(&exact).filter(|parameters| parameters.len() == call.args.len()) {
                            for (argument, parameter) in call.args.iter().zip(parameters) {
                                if let (Expr::Reference(reference), Some(value)) = (argument, parameter) {
                                    if reference.mutability.is_some() && matches!(reference.expr.as_ref(), Expr::Path(path) if path.path.is_ident(self.name)) {
                                        self.values.push(value.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                visit::visit_expr_call(self, call);
            }
        }
        let mut constraints = Constraints { index: self, name, values: Vec::new(), rebound: false, declarations: 0, bound: scope.keys().cloned().collect(), callees: BTreeSet::new() };
        constraints.visit_block(block);
        let first = constraints.values.first()?;
        (!constraints.rebound && constraints.bound.is_disjoint(&constraints.callees) && constraints.declarations == 1 && constraints.values.iter().all(|value| value == first)).then(|| first.clone())
    }

    pub(super) fn is_standard_empty_vec(&self, expression: &Expr) -> bool {
        let Expr::Call(call) = expression else { return false; };
        let Expr::Path(path) = call.func.as_ref() else { return false; };
        if !call.args.is_empty() || path.path.segments.len() != 2 || path.path.segments[0].ident != "Vec"
            || path.path.segments[1].ident != "new" || self.shadowed_types.contains("Vec") { return false; }
        let Some(file) = self.sources.get(&self.path).and_then(|source| syn::parse_file(source).ok()) else { return false; };
        fn glob(tree: &syn::UseTree) -> bool {
            match tree {
                syn::UseTree::Glob(_) => true,
                syn::UseTree::Path(path) => glob(&path.tree),
                syn::UseTree::Group(group) => group.items.iter().any(glob),
                _ => false,
            }
        }
        !super::rust_import_aliases(&file).contains_key("Vec") && !file.items.iter().any(|item|
            matches!(item, Item::Use(item) if glob(&item.tree)) || matches!(item, Item::Mod(item) if item.ident == "Vec"))
    }

    pub(super) fn expression_type(
        &self,
        expression: &Expr,
        values: &BTreeMap<String, RustValueType>,
    ) -> Option<RustValueType> {
        match expression {
            Expr::Path(path) => values.get(&path.path.get_ident()?.to_string()).cloned(),
            Expr::Reference(reference) => self.expression_type(&reference.expr, values),
            Expr::Paren(paren) => self.expression_type(&paren.expr, values),
            Expr::Tuple(tuple) => Some(RustValueType::Tuple(tuple.elems.iter().map(|expr|
                self.expression_type(expr, values).unwrap_or(RustValueType::Unknown)).collect())),
            Expr::Field(field) => {
                let RustValueType::Named(owner) = self.expression_type(&field.base, values)? else { return None; };
                let member = match &field.member {
                    syn::Member::Named(name) => name.to_string(),
                    syn::Member::Unnamed(index) => index.index.to_string(),
                };
                self.fields.get(&(owner, member)).cloned()
            }
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
                self.function_returns.get(&exact).cloned()
            }
            Expr::MethodCall(call) => {
                let receiver = self.expression_type(&call.receiver, values)?;
                let method = call.method.to_string();
                match receiver {
                    RustValueType::Named(name) => {
                        self.returns.get(&format!("{name}::{method}")).cloned()
                    }
                    RustValueType::Sequence(element) => match method.as_str() {
                        "iter" | "into_iter" => Some(RustValueType::Iterator(element)),
                        "to_vec" | "as_slice" | "clone" => Some(RustValueType::Sequence(element)),
                        "first" | "last" => Some(RustValueType::Optional(element)),
                        _ => None,
                    },
                    RustValueType::MapValues(element)
                        if matches!(method.as_str(), "values" | "into_values") =>
                    {
                        Some(RustValueType::Iterator(element))
                    }
                    RustValueType::Iterator(element)
                        if matches!(method.as_str(), "filter" | "take" | "skip" | "copied" | "cloned") =>
                    {
                        Some(RustValueType::Iterator(element))
                    }
                    RustValueType::Iterator(element) if matches!(method.as_str(), "find" | "min" | "max" | "min_by" | "max_by" | "min_by_key" | "max_by_key") => {
                        Some(RustValueType::Optional(element))
                    }
                    RustValueType::Optional(element) if matches!(method.as_str(), "filter" | "as_ref" | "copied" | "cloned" | "or_else") => {
                        Some(RustValueType::Optional(element))
                    }
                    RustValueType::Optional(element) if matches!(method.as_str(), "and_then" | "map") => {
                        let Expr::Closure(closure) = call.args.first()? else { return None; };
                        if closure.inputs.len() != 1 { return None; }
                        let Pat::Ident(name) = &closure.inputs[0] else { return None; };
                        let mut nested = values.clone();
                        nested.insert(name.ident.to_string(), *element);
                        let result = self.expression_type(&closure.body, &nested)?;
                        if method == "and_then" { Some(result) } else { Some(RustValueType::Optional(Box::new(result))) }
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

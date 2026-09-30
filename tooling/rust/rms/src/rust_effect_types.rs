//! Bounded source-declared receiver types. Unknown and ambiguous types stay unknown.
use std::collections::{BTreeMap, BTreeSet};
use syn::{Expr, FnArg, GenericArgument, Item, Pat, PathArguments, ReturnType, Signature, Type};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum RustValueType {
    Unknown,
    Text,
    Named(String), // Exact source-path#type identity, never a global leaf name.
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

    pub(super) fn callback_inputs(&self, method: &str, argument: usize) -> Option<Vec<Self>> {
        match (self, method, argument) {
            (Self::Optional(_), "map_or_else", 0) => Some(Vec::new()),
            (Self::Optional(element), "map_or_else", 1) => Some(vec![(**element).clone()]),
            (Self::Iterator(element), "fold", 1) => Some(vec![Self::Unknown, (**element).clone()]),
            (Self::Iterator(element), "flat_map", 0) => Some(vec![(**element).clone()]),
            (_, "map" | "filter" | "filter_map" | "any" | "all" | "find" | "and_then" | "is_some_and" | "max_by_key" | "min_by_key", 0) => {
                self.element().map(|element| vec![element.clone()])
            }
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
    variants: BTreeMap<(String, String), Vec<RustValueType>>,
    generic_names: BTreeSet<String>,
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
        for (path, file) in &files {
            for item in &file.items {
                let name = match item {
                    Item::Struct(item) => Some(&item.ident),
                    Item::Enum(item) => Some(&item.ident),
                    Item::Type(item) => Some(&item.ident),
                    _ => None,
                };
                if let Some(name) = name {
                    *counts.entry(format!("{path}#{name}")).or_default() += 1;
                }
            }
        }
        let mut index = Self {
            unique_types: counts
                .iter()
                .filter(|(_, count)| **count == 1)
                .map(|(name, _)| name.clone())
                .collect(),
            shadowed_types: counts.keys().map(|key| super::symbol_name(key).to_string()).collect(),
            returns: BTreeMap::new(),
            function_returns: BTreeMap::new(),
            fields: BTreeMap::new(),
            mutable_sequences: BTreeMap::new(),
            variants: BTreeMap::new(),
            generic_names: BTreeSet::new(),
            sources: sources.clone(),
            path: String::new(),
        };
        let mut signatures = BTreeMap::<String, Vec<(String, Type)>>::new();
        for (path, file) in &files {
            for item in &file.items {
                if let Item::Struct(item) = item {
                    if index.unique_types.contains(&format!("{path}#{}", item.ident)) && item.generics.params.is_empty()
                        && !item.attrs.iter().any(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr")) {
                        for (position, field) in item.fields.iter().enumerate() {
                            if !field.attrs.is_empty() { continue; }
                            if let Some(value) = index.for_path(path).parse_type(&field.ty) {
                                let name = field.ident.as_ref().map(ToString::to_string).unwrap_or_else(|| position.to_string());
                                index.fields.insert((format!("{path}#{}", item.ident), name), value);
                            }
                        }
                    }
                }
                if let Item::Enum(item) = item {
                    let owner = format!("{path}#{}", item.ident);
                    if index.unique_types.contains(&owner) && item.generics.params.is_empty()
                        && !item.attrs.iter().any(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr")) {
                        for variant in &item.variants {
                            if item.variants.iter().filter(|candidate| candidate.ident == variant.ident).count() != 1 { continue; }
                            let syn::Fields::Unnamed(fields) = &variant.fields else { continue; };
                            if !variant.attrs.is_empty() || fields.unnamed.iter().any(|field| !field.attrs.is_empty()) { continue; }
                            let values = fields.unnamed.iter().map(|field| index.for_path(path).parse_type(&field.ty).unwrap_or(RustValueType::Unknown)).collect();
                            index.variants.insert((owner.clone(), variant.ident.to_string()), values);
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
                let owner = format!("{path}#{owner}");
                if !index.unique_types.contains(&owner) {
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

    pub(super) fn with_generics(&self, generics: &syn::Generics) -> Self {
        let mut result = self.clone();
        result.generic_names.extend(generics.type_params().map(|parameter| parameter.ident.to_string()));
        result
    }

    pub(super) fn is_generic(&self, name: &str) -> bool { self.generic_names.contains(name) || self.generic_names.contains("*") }

    pub(super) fn is_declared_type(&self, name: &str) -> bool { self.shadowed_types.contains(name) }

    pub(super) fn with_local_type_shadows(&self, block: &syn::Block) -> Self {
        let mut result = self.clone();
        for statement in &block.stmts {
            let syn::Stmt::Item(item) = statement else { continue; };
            let name = match item {
                Item::Struct(item) => Some(item.ident.to_string()),
                Item::Enum(item) => Some(item.ident.to_string()),
                Item::Type(item) => Some(item.ident.to_string()),
                Item::Mod(item) => Some(item.ident.to_string()),
                Item::Use(item) => {
                    fn collect_names(tree: &syn::UseTree, names: &mut BTreeSet<String>) {
                        match tree {
                            syn::UseTree::Name(item) => { names.insert(item.ident.to_string()); }
                            syn::UseTree::Rename(item) => { names.insert(item.rename.to_string()); }
                            syn::UseTree::Glob(_) => { names.insert("*".into()); }
                            syn::UseTree::Path(item) => collect_names(&item.tree, names),
                            syn::UseTree::Group(group) => { for item in &group.items { collect_names(item, names); } }
                        }
                    }
                    collect_names(&item.tree, &mut result.generic_names);
                    None
                }
                _ => None,
            };
            if let Some(name) = name { result.generic_names.insert(name); }
        }
        result
    }

    pub(super) fn standard_str_available(&self, signature: &Signature, block: &syn::Block) -> bool {
        use syn::visit::{self, Visit};
        fn shadows(tree: &syn::UseTree) -> bool {
            match tree {
                syn::UseTree::Glob(_) => true,
                syn::UseTree::Name(name) => name.ident == "str",
                syn::UseTree::Rename(name) => name.rename == "str",
                syn::UseTree::Path(path) => shadows(&path.tree),
                syn::UseTree::Group(group) => group.items.iter().any(shadows),
            }
        }
        fn item_shadows(item: &Item) -> bool {
            match item {
                Item::Struct(item) => item.ident == "str",
                Item::Enum(item) => item.ident == "str",
                Item::Type(item) => item.ident == "str",
                Item::Mod(item) => item.ident == "str",
                Item::Trait(item) => item.ident == "str",
                Item::Impl(item) => item.generics.type_params().any(|parameter| parameter.ident == "str"),
                Item::ExternCrate(item) => item.rename.as_ref().map(|(_, name)| name == "str").unwrap_or(item.ident == "str"),
                Item::Use(item) => shadows(&item.tree),
                _ => false,
            }
        }
        let Some(file) = self.sources.get(&self.path).and_then(|source| syn::parse_file(source).ok()) else { return false; };
        if file.items.iter().any(item_shadows) || signature.generics.type_params().any(|parameter| parameter.ident == "str") { return false; }
        struct LocalShadows(bool);
        impl<'ast> Visit<'ast> for LocalShadows {
            fn visit_item(&mut self, item: &'ast Item) {
                self.0 |= item_shadows(item);
                visit::visit_item(self, item);
            }
            fn visit_type_param(&mut self, parameter: &'ast syn::TypeParam) {
                self.0 |= parameter.ident == "str";
            }
        }
        let mut local = LocalShadows(false);
        local.visit_block(block);
        !local.0
    }

    fn parse_type(&self, value: &Type) -> Option<RustValueType> {
        match value {
            Type::Reference(reference) => self.parse_type(&reference.elem),
            Type::Array(array) => Some(RustValueType::Sequence(Box::new(
                self.parse_type(&array.elem).unwrap_or(RustValueType::Unknown)))),
            Type::Tuple(tuple) => Some(RustValueType::Tuple(tuple.elems.iter()
                .map(|element| self.parse_type(element).unwrap_or(RustValueType::Unknown)).collect())),
            Type::Slice(slice) => Some(RustValueType::Sequence(Box::new(
                self.parse_type(&slice.elem).unwrap_or(RustValueType::Unknown)))),
            Type::Path(path) if path.qself.is_none() => {
                let segment = path.path.segments.last()?;
                let name = segment.ident.to_string();
                if path.path.segments.len() == 1 && matches!(name.as_str(), "str" | "String")
                    && self.unshadowed_external_root(&name) {
                    return Some(RustValueType::Text);
                }
                if path.path.segments.len() == 1 && self.is_generic(&name) { return None; }
                if matches!(segment.arguments, PathArguments::None) {
                    let reference = path
                        .path
                        .segments
                        .iter()
                        .map(|s| s.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    return self.resolve_named_type(&reference).map(RustValueType::Named);
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
                if name == "Vec" && aliases.contains_key(&name)
                    && (aliases.get(&name).map(String::as_str) != Some("std::vec::Vec")
                        || aliases.contains_key("std")
                        || file.items.iter().any(|item| matches!(item, Item::Mod(item) if item.ident == "std"))
                        || !self.standard_external_crate_available("std"))
                {
                    return None;
                }
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
                let element = Box::new(if name == "Vec" {
                    self.parse_type(element).unwrap_or(RustValueType::Unknown)
                } else { self.parse_type(element)? });
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

    pub(super) fn standard_external_crate_available(&self, root: &str) -> bool {
        use syn::visit::{self, Visit};
        struct Rebound<'a> { root: &'a str, found: bool }
        impl<'ast> Visit<'ast> for Rebound<'_> {
            fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
                let binding = item.rename.as_ref().map(|(_, name)| name).unwrap_or(&item.ident);
                self.found |= binding == self.root && item.ident != self.root;
                visit::visit_item_extern_crate(self, item);
            }
        }
        let mut rebound = Rebound { root, found: false };
        // A crate-root extern alias also affects calls in another source file.
        // Refuse ambiguous roots across the available Rust source closure.
        for (path, source) in &self.sources {
            if !path.ends_with(".rs") { continue; }
            let Ok(file) = syn::parse_file(source) else { return false; };
            rebound.visit_file(&file);
        }
        !rebound.found
    }

    pub(super) fn unshadowed_external_root(&self, root: &str) -> bool {
        if self.is_generic(root) || !self.standard_external_crate_available(root) { return false; }
        let Some(file) = self.sources.get(&self.path).and_then(|source| syn::parse_file(source).ok()) else { return false; };
        fn imported(tree: &syn::UseTree, name: &str) -> bool {
            match tree {
                syn::UseTree::Glob(_) => true,
                syn::UseTree::Name(item) => item.ident == name,
                syn::UseTree::Rename(item) => item.rename == name,
                syn::UseTree::Path(item) => imported(&item.tree, name),
                syn::UseTree::Group(group) => group.items.iter().any(|item| imported(item, name)),
            }
        }
        !file.items.iter().any(|item| match item {
            Item::Mod(item) => item.ident == root,
            Item::Struct(item) => item.ident == root,
            Item::Enum(item) => item.ident == root,
            Item::Type(item) => item.ident == root,
            Item::Trait(item) => item.ident == root,
            Item::Use(item) => imported(&item.tree, root),
            _ => false,
        })
    }

    pub(super) fn standard_integer_available(&self, name: &str) -> bool {
        if !matches!(name, "u8" | "u16" | "u32" | "u64" | "u128" | "usize" |
            "i8" | "i16" | "i32" | "i64" | "i128" | "isize") || self.is_generic(name) { return false; }
        use syn::visit::{self, Visit};
        struct Shadows<'a> { name: &'a str, found: bool, globs: Vec<String> }
        impl Shadows<'_> {
            fn imports(&mut self, tree: &syn::UseTree, prefix: &str) {
                match tree {
                    syn::UseTree::Name(item) => self.found |= item.ident == self.name,
                    syn::UseTree::Rename(item) => self.found |= item.rename == self.name,
                    syn::UseTree::Path(item) => self.imports(&item.tree, &format!("{prefix}{}::", item.ident)),
                    syn::UseTree::Group(group) => { for item in &group.items { self.imports(item, prefix); } }
                    syn::UseTree::Glob(_) => self.globs.push(prefix.trim_end_matches("::").into()),
                }
            }
        }
        impl<'ast> Visit<'ast> for Shadows<'_> {
            fn visit_item(&mut self, item: &'ast Item) {
                self.found |= match item {
                    Item::Struct(item) => item.ident == self.name,
                    Item::Enum(item) => item.ident == self.name,
                    Item::Type(item) => item.ident == self.name,
                    Item::Trait(item) => item.ident == self.name,
                    Item::Mod(item) => item.ident == self.name,
                    Item::ExternCrate(item) => item.rename.as_ref().map(|(_, id)| id).unwrap_or(&item.ident) == self.name,
                    _ => false,
                };
                if let Item::Use(item) = item { self.imports(&item.tree, ""); }
                visit::visit_item(self, item);
            }
            fn visit_type_param(&mut self, parameter: &'ast syn::TypeParam) {
                self.found |= parameter.ident == self.name;
            }
        }
        let mut pending = vec![self.path.clone()];
        let mut seen = BTreeSet::new();
        while let Some(path) = pending.pop() {
            if !seen.insert(path.clone()) { continue; }
            let Some(file) = self.sources.get(&path).and_then(|source| syn::parse_file(source).ok()) else { return false; };
            let mut shadows = Shadows { name, found: false, globs: Vec::new() };
            shadows.visit_file(&file);
            if shadows.found { return false; }
            for glob in shadows.globs {
                // Only exact source-owned crate-module globs are inspectable here.
                let Some(module) = glob.strip_prefix("crate::").filter(|module| !module.contains("::")) else { return false; };
                let target = format!("src/{module}.rs");
                if !self.sources.contains_key(&target) { return false; }
                pending.push(target);
            }
        }
        true
    }

    fn resolve_named_type(&self, reference: &str) -> Option<String> {
        fn imported_count(tree: &syn::UseTree, name: &str) -> usize {
            match tree {
                syn::UseTree::Name(item) => usize::from(item.ident == name),
                syn::UseTree::Rename(item) => usize::from(item.rename == name),
                syn::UseTree::Path(item) => imported_count(&item.tree, name),
                syn::UseTree::Group(group) => group.items.iter().map(|item| imported_count(item, name)).sum(),
                syn::UseTree::Glob(_) => 0,
            }
        }
        let imports = |file: &syn::File, name: &str| {
            let mut count = 0;
            let mut conditional = false;
            for item in &file.items {
                if let Item::Use(item) = item {
                    let current = imported_count(&item.tree, name);
                    count += current;
                    conditional |= current > 0 && !item.attrs.is_empty();
                }
            }
            (count, conditional)
        };
        if !reference.contains("::") {
            let file = syn::parse_file(self.sources.get(&self.path)?).ok()?;
            let (count, conditional) = imports(&file, reference);
            if count > 1 || conditional { return None; }
        }
        let exact = super::rust_exact_item_reference(&self.path, reference, &self.sources, 0)?;
        if !self.unique_types.contains(&exact) { return None; }
        let (path, name) = exact.split_once('#')?;
        let file = syn::parse_file(self.sources.get(path)?).ok()?;
        // A declaration plus an explicit same-name import is not a unique
        // binding. Do not let the resolver's local-item preference hide it.
        if imports(&file, name).0 > 0 { return None; }
        Some(exact)
    }

    pub(super) fn parameters(&self, signature: &Signature) -> BTreeMap<String, RustValueType> {
        let mut scope = self.clone();
        scope.generic_names.extend(signature
            .generics
            .type_params()
            .map(|parameter| parameter.ident.to_string())
            .collect::<BTreeSet<_>>());
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
                let value = scope
                    .parse_type(&argument.ty)
                    .unwrap_or(RustValueType::Unknown);
                Some((name.ident.to_string(), value))
            })
            .collect()
    }

    pub(super) fn pattern_bindings(&self, pattern: &Pat, value: &RustValueType) -> BTreeMap<String, RustValueType> {
        let mut bindings = BTreeMap::new();
        match (pattern, value) {
            (Pat::Ident(name), value) => { bindings.insert(name.ident.to_string(), value.clone()); }
            (Pat::Reference(pattern), value) => { return self.pattern_bindings(&pattern.pat, value); }
            (Pat::Type(pattern), value) => { return self.pattern_bindings(&pattern.pat, value); }
            (Pat::TupleStruct(pattern), RustValueType::Optional(element))
                if pattern.path.is_ident("Some") && pattern.elems.len() == 1 && !self.is_generic("Some") => {
                return self.pattern_bindings(&pattern.elems[0], element);
            }
            (Pat::TupleStruct(pattern), RustValueType::Named(owner)) => {
                if pattern.qself.is_some() || pattern.path.segments.iter().any(|segment| !matches!(segment.arguments, PathArguments::None)) { return bindings; }
                let mut path = pattern.path.clone();
                let Some(variant) = path.segments.pop() else { return bindings; };
                if path.segments.is_empty() { return bindings; }
                let reference = path.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>().join("::");
                if path.segments.first().is_some_and(|segment| self.is_generic(&segment.ident.to_string())) { return bindings; }
                if self.resolve_named_type(&reference).as_ref() != Some(owner) { return bindings; }
                if let Some(values) = self.variants.get(&(owner.clone(), variant.value().ident.to_string()))
                    .filter(|values| values.len() == pattern.elems.len()) {
                    for (pattern, value) in pattern.elems.iter().zip(values) { bindings.extend(self.pattern_bindings(pattern, value)); }
                }
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

    pub(super) fn sequence_annotation(&self, pattern: &Pat) -> Option<RustValueType> {
        let Pat::Type(pattern) = pattern else { return None; };
        self.parse_type(&pattern.ty).filter(|value| matches!(value, RustValueType::Sequence(_)))
    }

    pub(super) fn inherent_self_type(&self, ty: &Type) -> Option<RustValueType> {
        let Type::Path(path) = ty else { return None; };
        if path.qself.is_some() || path.path.segments.len() != 1 { return None; }
        let segment = &path.path.segments[0];
        if self.is_generic(&segment.ident.to_string()) { return None; }
        match &segment.arguments {
            PathArguments::None => {},
            PathArguments::AngleBracketed(arguments) if arguments.args.iter().all(|arg| matches!(arg, GenericArgument::Lifetime(_))) => {},
            _ => return None,
        }
        let exact = format!("{}#{}", self.path, segment.ident);
        self.unique_types.contains(&exact).then_some(RustValueType::Named(exact))
    }

    fn local_constructed_type(&self, path: &syn::Path, tuple_arity: Option<usize>) -> Option<RustValueType> {
        if path.segments.len() != 1 || path.leading_colon.is_some() { return None; }
        let name = path.segments[0].ident.to_string();
        if self.is_generic(&name) { return None; }
        let file = syn::parse_file(self.sources.get(&self.path)?).ok()?;
        if super::rust_import_aliases(&file).contains_key(&name)
            || file.items.iter().any(|item| matches!(item, Item::Fn(item) if item.sig.ident == name)) { return None; }
        let exact = format!("{}#{name}", self.path);
        if !self.unique_types.contains(&exact) { return None; }
        let item = file.items.iter().find_map(|item| match item {
            Item::Struct(item) if item.ident == name => Some(item), _ => None,
        })?;
        if item.generics.type_params().next().is_some() || item.generics.const_params().next().is_some()
            || item.attrs.iter().any(|attr| attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr")) { return None; }
        match (&item.fields, tuple_arity) {
            (syn::Fields::Unnamed(fields), Some(arity)) if fields.unnamed.len() == arity => {},
            (syn::Fields::Named(_), None) => {},
            _ => return None,
        }
        Some(RustValueType::Named(exact))
    }

    pub(super) fn expression_type(
        &self,
        expression: &Expr,
        values: &BTreeMap<String, RustValueType>,
    ) -> Option<RustValueType> {
        match expression {
            Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_)) => Some(RustValueType::Text),
            Expr::Path(path) => values.get(&path.path.get_ident()?.to_string()).cloned(),
            Expr::Reference(reference) => self.expression_type(&reference.expr, values),
            Expr::Paren(paren) => self.expression_type(&paren.expr, values),
            Expr::Index(index) if matches!(index.index.as_ref(), Expr::Range(_))
                && self.expression_type(&index.expr, values) == Some(RustValueType::Text) => Some(RustValueType::Text),
            Expr::Tuple(tuple) => Some(RustValueType::Tuple(tuple.elems.iter().map(|expr|
                self.expression_type(expr, values).unwrap_or(RustValueType::Unknown)).collect())),
            Expr::Struct(value) if value.qself.is_none() => self.local_constructed_type(&value.path, None),
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
                if path.qself.is_none() {
                    if let Some(value) = self.local_constructed_type(&path.path, Some(call.args.len())) {
                        return Some(value);
                    }
                }
                let reference = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                if reference == "std::str::from_utf8" && call.args.len() == 1
                    && self.unshadowed_external_root("std") {
                    return Some(RustValueType::ResultOk(Box::new(RustValueType::Text)));
                }
                let exact =
                    super::rust_exact_function_reference(&self.path, &reference, &self.sources, 0)?;
                self.function_returns.get(&exact).cloned()
            }
            Expr::MethodCall(call) => {
                let receiver = self.expression_type(&call.receiver, values)?;
                let method = call.method.to_string();
                match receiver {
                    RustValueType::ResultOk(element) if method == "map_err" && call.args.len() == 1 =>
                        Some(RustValueType::ResultOk(element)),
                    RustValueType::Text => match method.as_str() {
                        "split_inclusive" | "split" | "lines" | "split_whitespace" =>
                            Some(RustValueType::Iterator(Box::new(RustValueType::Text))),
                        "as_str" | "to_owned" | "to_string" | "clone" | "trim" | "trim_end_matches" => Some(RustValueType::Text),
                        _ => None,
                    },
                    RustValueType::Named(name) => {
                        self.returns.get(&format!("{name}::{method}")).cloned()
                    }
                    RustValueType::Sequence(element) => match method.as_str() {
                        "iter" | "into_iter" => Some(RustValueType::Iterator(element)),
                        "to_vec" | "as_slice" | "clone" => Some(RustValueType::Sequence(element)),
                        "first" | "last" | "last_mut" => Some(RustValueType::Optional(element)),
                        _ => None,
                    },
                    RustValueType::MapValues(element)
                        if matches!(method.as_str(), "values" | "into_values") =>
                    {
                        Some(RustValueType::Iterator(element))
                    }
                    RustValueType::Iterator(element)
                        if method == "enumerate" && call.args.is_empty() =>
                    {
                        Some(RustValueType::Iterator(Box::new(RustValueType::Tuple(vec![RustValueType::Unknown, *element]))))
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

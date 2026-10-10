//! Exact, source-local call identity for trace proof inspection.
//! This index does not infer effects or trust names from unrelated source files.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use syn::{Item, UseTree};
use syn::visit::{self, Visit};

pub(crate) struct Index {
    files: BTreeMap<PathBuf, syn::File>,
    library: PathBuf,
    crate_name: Option<String>,
    test_parents: Option<BTreeMap<PathBuf, Vec<PathBuf>>>,
}

impl Index {
    pub(crate) fn from_parsed(files: BTreeMap<PathBuf, syn::File>, library: PathBuf, crate_name: Option<String>) -> Self {
        Self { files: files.into_iter().filter_map(|(path, file)| normalize(&path).map(|path| (path, file))).collect(), library: normalize(&library).unwrap_or(library), crate_name, test_parents: None }
    }

    pub(crate) fn new(
        sources: &BTreeMap<PathBuf, String>,
        library: PathBuf,
        crate_name: Option<String>,
    ) -> Self {
        Self {
            files: sources
                .iter()
                .filter_map(|(path, source)| {
                    syn::parse_file(source)
                        .ok()
                        .map(|file| (path.clone(), file))
                })
                .collect(),
            library,
            crate_name,
            test_parents: None,
        }
    }

    /// Only trace proofs run in the explicit test configuration. Production
    /// call inspection retains its existing conditional-declaration policy.
    pub(crate) fn for_test_proof(mut self, runner: &Path) -> Self {
        self.test_parents = Some(BTreeMap::new());
        let mut parents = BTreeMap::<PathBuf, Vec<PathBuf>>::new();
        let mut visited = BTreeSet::new();
        // A helper can be compiled by several independent integration crates.
        // Only attachments reachable from this library and selected test root
        // participate in this proof; duplicate active attachments still fail.
        for root in [self.library.clone(), runner.to_path_buf()] {
            let mut pending = vec![root];
            while let Some(path) = pending.pop() {
                if !visited.insert(path.clone()) { continue; }
                let Some(file) = self.files.get(&path) else { continue; };
                for item in &file.items {
                    if let Item::Mod(module) = item {
                        if let Some(target) = self.module_target(&path, module) {
                            parents.entry(target.clone()).or_default().push(path.clone());
                            pending.push(target);
                        }
                    }
                }
            }
        }
        self.test_parents = Some(parents);
        self
    }

    fn proof_parent(&self, path: &Path) -> Result<Option<&PathBuf>, ()> {
        match self.test_parents.as_ref().and_then(|parents| parents.get(path)) {
            None => Ok(None),
            Some(parents) if parents.len() == 1 => Ok(parents.first()),
            _ => Err(()),
        }
    }

    fn proof_root<'a>(&'a self, path: &'a Path) -> Result<&'a Path, ()> {
        let mut root = path;
        let mut seen = BTreeSet::new();
        while let Some(parent) = self.proof_parent(root)? {
            if !seen.insert(root.to_path_buf()) || seen.len() > 24 { return Err(()); }
            root = parent;
        }
        Ok(root)
    }

    fn module_target(&self, path: &Path, module: &syn::ItemMod) -> Option<PathBuf> {
        if module.content.is_some() { return None; }
        let mut explicit = Vec::new();
        for attr in &module.attrs {
            if let syn::Meta::NameValue(meta) = &attr.meta {
                if meta.path.is_ident("path") {
                    if let syn::Expr::Lit(value) = &meta.value {
                        if let syn::Lit::Str(value) = &value.lit { explicit.push(value.value()); continue; }
                    }
                }
            }
            let test_cfg = self.test_parents.is_some() && attr.path().is_ident("cfg")
                && attr.parse_args::<syn::Path>().is_ok_and(|path| path.is_ident("test"));
            if !attr.path().is_ident("doc") && !test_cfg { return None; }
        }
        let parent = path.parent()?;
        let candidates = match explicit.as_slice() {
            [relative] => vec![normalize(&parent.join(relative))?],
            [] => {
                let directory = if matches!(path.file_name().and_then(|name| name.to_str()), Some("lib.rs" | "main.rs" | "mod.rs"))
                    || path.parent().and_then(Path::file_name).and_then(|name| name.to_str()) == Some("tests") {
                    parent.to_path_buf()
                } else { parent.join(path.file_stem()?) };
                vec![directory.join(format!("{}.rs", module.ident)), directory.join(module.ident.to_string()).join("mod.rs")]
            }
            _ => return None,
        };
        let mut existing = candidates.into_iter().filter(|path| self.files.contains_key(path));
        let target = existing.next()?;
        if existing.next().is_some() { return None; }
        Some(target)
    }

    pub(crate) fn resolve(&self, path: &Path, call: &[String]) -> Option<(PathBuf, String)> {
        self.resolve_at(path, call, 0).ok()
    }

    /// Positive reachability requires exact source identities on every edge.
    pub(crate) fn reaches(&self, start: (PathBuf, String), target: (PathBuf, String)) -> bool {
        let Some(target_path) = normalize(&target.0) else { return false; };
        let target = (target_path, target.1);
        self.reaches_where(start, |path, symbol, _| path == target.0 && symbol == target.1)
    }

    pub(crate) fn reaches_control_loop(&self, start: (PathBuf, String)) -> bool {
        #[derive(Default)]
        struct Loops(bool);
        impl<'ast> Visit<'ast> for Loops {
            fn visit_expr_loop(&mut self, _: &'ast syn::ExprLoop) { self.0 = true; }
            fn visit_expr_while(&mut self, _: &'ast syn::ExprWhile) { self.0 = true; }
            fn visit_expr_for_loop(&mut self, _: &'ast syn::ExprForLoop) { self.0 = true; }
            fn visit_item(&mut self, _: &'ast Item) {}
            fn visit_expr_closure(&mut self, _: &'ast syn::ExprClosure) {}
        }
        self.reaches_where(start, |_, _, body| { let mut loops = Loops::default(); loops.visit_block(body); loops.0 })
    }

    fn reaches_where(&self, start: (PathBuf, String), predicate: impl Fn(&Path, &str, &syn::Block) -> bool) -> bool {
        let Some(start_path) = normalize(&start.0) else { return false; };
        let start = (start_path, start.1);
        let mut pending = vec![start];
        let mut seen = BTreeSet::new();
        while let Some((path, symbol)) = pending.pop() {
            if !seen.insert((path.clone(), symbol.clone())) { continue; }
            if seen.len() > 512 { return false; }
            let Some(file) = self.files.get(&path) else { continue; };
            let identity = symbol.split("::").map(str::to_string).collect::<Vec<_>>();
            if self.resolve(&path, &identity) != Some((path.clone(), symbol.clone())) { continue; }
            let mut bodies = Vec::new();
            for item in &file.items {
                match item {
                    Item::Fn(function) if function.sig.ident == symbol && function.attrs.iter().all(|attr| attr.path().is_ident("doc") || attr.path().is_ident("test")) => bodies.push((None, &function.sig, &*function.block)),
                    Item::Impl(block) if block.trait_.is_none() && block.attrs.is_empty() => {
                        let syn::Type::Path(ty) = &*block.self_ty else { continue; };
                        let Some(name) = ty.path.get_ident() else { continue; };
                        for item in &block.items {
                            if let syn::ImplItem::Fn(function) = item {
                                if symbol == format!("{name}::{}", function.sig.ident) && function.attrs.iter().all(|attr| attr.path().is_ident("doc")) {
                                    bodies.push((Some(name.to_string()), &function.sig, &function.block));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            if bodies.len() != 1 { continue; }
            let (self_type, signature, body) = bodies[0].clone();
            if predicate(&path, &symbol, body) { return true; }
            let mut collector = Calls { index: self, path: &path, self_type, locals: BTreeMap::new(), calls: Vec::new(), local_imports: BTreeSet::new() };
            for input in &signature.inputs {
                if let syn::FnArg::Typed(input) = input {
                    if let syn::Pat::Ident(name) = &*input.pat { collector.locals.insert(name.ident.to_string(), type_path(&input.ty)); }
                }
            }
            collector.visit_block(body);
            pending.extend(collector.calls);
        }
        false
    }

    fn resolve_at(
        &self,
        path: &Path,
        call: &[String],
        depth: usize,
    ) -> Result<(PathBuf, String), ()> {
        if depth > 24 || call.is_empty() {
            return Err(());
        }
        let first = &call[0];
        if self.test_parents.is_some() { self.proof_root(path)?; }
        let file = self.files.get(path).ok_or(())?;
        let mut root_imports = Vec::new();
        for item in &file.items {
            if let Item::Use(item) = item {
                collect_imports(&item.tree, &[], first, &mut root_imports, &mut Vec::new());
            }
        }
        let local_root = !root_imports.is_empty()
            || file
                .items
                .iter()
                .any(|item| matches!(item, Item::Mod(module) if module.ident == first));
        if call.len() > 1 {
            if self.crate_name.as_ref() == Some(first) && !local_root {
                return self.resolve_at(&self.library, &call[1..], depth + 1);
            }
            if first == "self" {
                return self.resolve_at(path, &call[1..], depth + 1);
            }
            if first == "crate" {
                // An integration test is its own crate, not the package library.
                let root = if self.test_parents.is_some() {
                    self.proof_root(path)?
                } else if path.starts_with(self.library.parent().ok_or(())?) {
                    self.library.as_path()
                } else {
                    path
                };
                return self.resolve_at(root, &call[1..], depth + 1);
            }
            if first == "super" && self.test_parents.is_some() {
                return self.resolve_at(self.proof_parent(path)?.ok_or(())?, &call[1..], depth + 1);
            }
        }
        let definitions = file
            .items
            .iter()
            .filter(|item| match item {
                Item::Fn(item) => item.sig.ident == first,
                Item::Mod(item) => item.ident == first,
                Item::Struct(item) => item.ident == first,
                Item::Enum(item) => item.ident == first,
                _ => false,
            })
            .collect::<Vec<_>>();
        if !definitions.is_empty() {
            // A module and a re-exported function can share a spelling. Rust
            // keeps their namespaces distinct; a qualified call uses the module.
            if definitions.len() != 1 || (!root_imports.is_empty() && !matches!(definitions[0], Item::Mod(_))) {
                return Err(());
            }
            return match definitions[0] {
                Item::Struct(_) | Item::Enum(_) if call.len() == 2 => {
                    let (attrs, generics) = match definitions[0] {
                        Item::Struct(owner) => (&owner.attrs, &owner.generics),
                        Item::Enum(owner) => (&owner.attrs, &owner.generics),
                        _ => unreachable!(),
                    };
                    if !generics.params.is_empty() || generics.where_clause.is_some()
                        || attrs.iter().any(|attr| !attr.path().is_ident("doc") && !attr.path().is_ident("derive")) { return Err(()); }
                    let mut methods = 0;
                    for item in &file.items {
                        let Item::Impl(block) = item else { continue; };
                        if block.trait_.is_some() { continue; }
                        let syn::Type::Path(ty) = block.self_ty.as_ref() else { continue; };
                        if !ty.path.segments.last().is_some_and(|part| part.ident == first) { continue; }
                        for member in &block.items {
                            let syn::ImplItem::Fn(function) = member else { continue; };
                            if function.sig.ident != call[1] { continue; }
                            if ty.qself.is_some() || !ty.path.is_ident(first)
                                || !block.generics.params.is_empty() || block.generics.where_clause.is_some()
                                || block.attrs.iter().any(|attr| !attr.path().is_ident("doc"))
                                || !function.sig.generics.params.is_empty() || function.sig.generics.where_clause.is_some()
                                || function.attrs.iter().any(|attr| !attr.path().is_ident("doc")) { return Err(()); }
                            methods += 1;
                        }
                    }
                    if methods == 1 { Ok((path.to_path_buf(), call.join("::"))) } else { Err(()) }
                }
                Item::Fn(item)
                    if call.len() == 1
                        && item.sig.generics.params.is_empty()
                        && item.sig.generics.where_clause.is_none()
                        && item.attrs.iter().all(|attr| {
                            attr.path().is_ident("test") || attr.path().is_ident("doc")
                        }) =>
                {
                    Ok((path.to_path_buf(), first.clone()))
                }
                Item::Mod(item) if call.len() > 1 && item.content.is_none() => {
                    let target = self.module_target(path, item).ok_or(())?;
                    self.resolve_at(&target, &call[1..], depth + 1)
                }
                _ => Err(()),
            };
        }
        let mut imports = Vec::new();
        let mut globs = Vec::new();
        for item in &file.items {
            if let Item::Use(item) = item {
                if !item.attrs.is_empty() {
                    continue;
                }
                collect_imports(&item.tree, &[], first, &mut imports, &mut globs);
            }
        }
        if !imports.is_empty() {
            if imports.len() != 1 {
                return Err(());
            }
            let mut target = imports.remove(0);
            target.extend_from_slice(&call[1..]);
            return self.resolve_at(path, &target, depth + 1);
        }
        let mut resolved = BTreeSet::new();
        for mut glob in globs {
            // Only known local modules or the exact package facade can supply a
            // unique name. An unknown external glob makes identity ambiguous.
            let known = glob.first().is_some_and(|name| {
                name == "crate"
                    || name == "self"
                    || (name == "super" && self.test_parents.is_some())
                    || self.crate_name.as_ref() == Some(name)
                    || file
                        .items
                        .iter()
                        .any(|item| matches!(item, Item::Mod(module) if module.ident == name))
            });
            if !known {
                return Err(());
            }
            glob.extend_from_slice(call);
            if let Ok(target) = self.resolve_at(path, &glob, depth + 1) {
                resolved.insert(target);
            }
        }
        if resolved.len() == 1 {
            Ok(resolved.into_iter().next().unwrap())
        } else {
            Err(())
        }
    }
}

fn type_path(ty: &syn::Type) -> Option<Vec<String>> {
    match ty {
        syn::Type::Reference(reference) => type_path(&reference.elem),
        syn::Type::Path(path) if path.qself.is_none()
            && path.path.segments.iter().all(|part| matches!(part.arguments, syn::PathArguments::None)) => Some(path.path.segments.iter().map(|part| part.ident.to_string()).collect()),
        _ => None,
    }
}

struct Calls<'a> {
    index: &'a Index,
    path: &'a Path,
    self_type: Option<String>,
    locals: BTreeMap<String, Option<Vec<String>>>,
    calls: Vec<(PathBuf, String)>,
    local_imports: BTreeSet<String>,
}

impl Calls<'_> {
    fn add(&mut self, mut path: Vec<String>) {
        if path.first().is_some_and(|root| !matches!(root.as_str(), "crate" | "self" | "super")
            && (self.local_imports.contains(root) || self.local_imports.contains("*"))) { return; }
        if path.first().is_some_and(|name| name == "Self") {
            let Some(name) = &self.self_type else { return; };
            path[0] = name.clone();
        } else if path.first().is_some_and(|name| self.locals.contains_key(name)) { return; }
        if let Some(call) = self.index.resolve(self.path, &path) { self.calls.push(call); }
    }
    fn value_type(&self, expr: &syn::Expr) -> Option<Vec<String>> {
        match expr {
            syn::Expr::Struct(value) => type_path(&syn::Type::Path(syn::TypePath { qself: value.qself.clone(), path: value.path.clone() })).map(|parts| parts.into_iter().map(|part| if part == "Self" { self.self_type.clone().unwrap_or_default() } else { part }).collect()),
            syn::Expr::Path(path) => {
                let name = path.path.get_ident()?.to_string();
                if name == "self" { self.self_type.clone().map(|name| vec![name]) } else { self.locals.get(&name).cloned().flatten() }
            }
            syn::Expr::Reference(value) => self.value_type(&value.expr),
            syn::Expr::Paren(value) => self.value_type(&value.expr),
            _ => None,
        }
    }
}

impl<'ast> Visit<'ast> for Calls<'_> {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = &*node.func {
            if path.qself.is_none() && path.path.segments.iter().all(|part| matches!(part.arguments, syn::PathArguments::None)) { self.add(path.path.segments.iter().map(|part| part.ident.to_string()).collect()); }
        }
        visit::visit_expr_call(self, node);
    }
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if let Some(mut ty) = self.value_type(&node.receiver) {
            ty.push(node.method.to_string());
            self.add(ty);
        }
        visit::visit_expr_method_call(self, node);
    }
    fn visit_local(&mut self, node: &'ast syn::Local) {
        visit::visit_local(self, node);
        if let syn::Pat::Ident(name) = &node.pat {
            let ty = node.init.as_ref().and_then(|init| self.value_type(&init.expr));
            self.locals.insert(name.ident.to_string(), ty);
        }
    }
    fn visit_block(&mut self, node: &'ast syn::Block) {
        let saved = self.locals.clone();
        let saved_imports = self.local_imports.clone();
        fn names(tree: &syn::UseTree, bound: &mut BTreeSet<String>) {
            match tree {
                syn::UseTree::Name(item) => { bound.insert(item.ident.to_string()); }
                syn::UseTree::Rename(item) => { bound.insert(item.rename.to_string()); }
                syn::UseTree::Path(item) => names(&item.tree, bound),
                syn::UseTree::Group(group) => { for item in &group.items { names(item, bound); } }
                syn::UseTree::Glob(_) => { bound.insert("*".into()); }
            }
        }
        for statement in &node.stmts {
            if let syn::Stmt::Item(Item::Fn(function)) = statement { self.locals.insert(function.sig.ident.to_string(), None); }
            if let syn::Stmt::Item(Item::Use(item)) = statement { names(&item.tree, &mut self.local_imports); }
            if let syn::Stmt::Item(item) = statement {
                let name = match item {
                    Item::Struct(item) => Some(&item.ident), Item::Enum(item) => Some(&item.ident),
                    Item::Type(item) => Some(&item.ident), Item::Mod(item) => Some(&item.ident),
                    Item::Trait(item) => Some(&item.ident), Item::Union(item) => Some(&item.ident),
                    Item::ExternCrate(item) => Some(item.rename.as_ref().map(|(_, name)| name).unwrap_or(&item.ident)),
                    _ => None,
                };
                if let Some(name) = name { self.local_imports.insert(name.to_string()); }
            }
        }
        visit::visit_block(self, node);
        self.locals = saved;
        self.local_imports = saved_imports;
    }
    fn visit_item(&mut self, node: &'ast Item) {
        // Local functions/imports are not executed by being declared.
        if let Item::Fn(function) = node { self.locals.insert(function.sig.ident.to_string(), None); }
    }
    fn visit_expr_closure(&mut self, _node: &'ast syn::ExprClosure) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherent_dispatch_preserves_exact_executor_paths_and_closed_owners() {
        let sources = BTreeMap::from([
            (PathBuf::from("/p/src/lib.rs"), "mod driver; mod runtime; mod a; mod b; pub use crate::a::a;".into()),
            (PathBuf::from("/p/src/driver.rs"), "use crate::runtime::Adapters; fn drive(adapters: &Adapters) { adapters.dispatch(); }".into()),
            (PathBuf::from("/p/src/runtime.rs"), "struct Adapters; struct NativePorts; enum Request { Run } impl Adapters { fn dispatch(&self) { NativePorts::dispatch(); } } impl NativePorts { fn dispatch() { let pending = async { use Request as R; crate::a::execute(); }; } }".into()),
            (PathBuf::from("/p/src/a.rs"), "fn execute() {} pub fn a() {}".into()),
            (PathBuf::from("/p/src/b.rs"), "fn execute() {}".into()),
        ]);
        let start = (PathBuf::from("/p/src/driver.rs"), "drive".into());
        let target = (PathBuf::from("/p/src/a.rs"), "execute".into());
        let index = Index::new(&sources, PathBuf::from("/p/src/lib.rs"), None);
        assert!(index.reaches(start.clone(), target.clone()));
        assert!(!index.reaches(start.clone(), (PathBuf::from("/p/src/b.rs"), "execute".into())));
        assert!(!index.reaches_control_loop(start.clone()));
        let mut looped = sources.clone();
        looped.insert(PathBuf::from("/p/src/b.rs"), "fn execute() { loop {} }".into());
        assert!(!Index::new(&looped, PathBuf::from("/p/src/lib.rs"), None).reaches_control_loop(start.clone()));
        looped.insert(PathBuf::from("/p/src/a.rs"), "fn execute() { loop {} }".into());
        assert!(Index::new(&looped, PathBuf::from("/p/src/lib.rs"), None).reaches_control_loop(start.clone()));
        let runtime = sources.get(&PathBuf::from("/p/src/runtime.rs")).unwrap();
        for invalid in [
            runtime.replace("impl NativePorts", "#[cfg(feature = \"optional\")] impl NativePorts"),
            runtime.replace("struct NativePorts;", "#[cfg(feature = \"optional\")] struct NativePorts;"),
            runtime.replace("impl NativePorts", "impl<T> NativePorts<T>"),
            runtime.replace("impl NativePorts", "impl other::NativePorts"),
            runtime.replace("impl NativePorts", "impl Trait for NativePorts"),
            runtime.replace("NativePorts::dispatch();", "NativePorts::<u8>::dispatch();"),
            runtime.replace("NativePorts::dispatch();", "struct NativePorts; NativePorts::dispatch();"),
            format!("{runtime} impl NativePorts {{ fn dispatch() {{}} }}"),
            format!("{runtime} #[cfg(feature = \"optional\")] impl NativePorts {{ fn dispatch() {{}} }}"),
            format!("{runtime} struct NativePorts;"),
        ] {
            let mut changed = sources.clone();
            changed.insert(PathBuf::from("/p/src/runtime.rs"), invalid.clone());
            assert!(!Index::new(&changed, PathBuf::from("/p/src/lib.rs"), None).reaches(start.clone(), target.clone()), "{invalid}");
        }
    }

    #[test]
    fn nested_crate_facade_methods_reach_only_the_exact_driver() {
        let sources = BTreeMap::from([
            (PathBuf::from("/p/src/lib.rs"), "mod repo; mod driver; pub use crate::repo::Repo;".to_string()),
            (PathBuf::from("/p/src/repo.rs"), "pub struct Repo {} impl Repo { pub fn open() { Self::create(); } fn create() { let repo = Self {}; repo.dispatch(); } fn dispatch(&self) { crate::driver::drive(); } }".to_string()),
            (PathBuf::from("/p/src/driver.rs"), "pub fn drive() {}".to_string()),
            (PathBuf::from("/p/cli/src/main.rs"), "use memory::Repo; fn run() { Repo::open(); }".to_string()),
        ]);
        let start = (PathBuf::from("/p/cli/src/main.rs"), "run".to_string());
        let target = (PathBuf::from("/p/src/driver.rs"), "drive".to_string());
        let index = Index::new(&sources, PathBuf::from("/p/src/lib.rs"), Some("memory".into()));
        assert!(index.reaches(start.clone(), target.clone()));
        for replacement in [
            "use other::Repo; fn run() { Repo::open(); }",
            "use memory::Repo; fn run() { let Repo = unknown; Repo::open(); }",
            "use memory::Repo; fn run() { let unused = || Repo::open(); }",
            "fn run() { use other::drive; drive(); }",
            "fn run() { drive(); fn drive() {} }",
            "fn run() { \"Repo::open()\"; }",
        ] {
            let mut changed = sources.clone();
            changed.insert(start.0.clone(), replacement.to_string());
            let index = Index::new(&changed, PathBuf::from("/p/src/lib.rs"), Some("memory".into()));
            assert!(!index.reaches(start.clone(), target.clone()), "{replacement}");
        }
        let mut changed = sources.clone();
        changed.insert(PathBuf::from("/p/src/repo.rs"), "pub struct Repo {} impl Repo { pub fn open() { unrelated::drive(); } }".into());
        let index = Index::new(&changed, PathBuf::from("/p/src/lib.rs"), Some("memory".into()));
        assert!(!index.reaches(start, target));
    }
}

fn normalize(path: &Path) -> Option<PathBuf> {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    return None;
                }
            }
            part => result.push(part.as_os_str()),
        }
    }
    Some(result)
}

fn collect_imports(
    tree: &UseTree,
    prefix: &[String],
    name: &str,
    imports: &mut Vec<Vec<String>>,
    globs: &mut Vec<Vec<String>>,
) {
    match tree {
        UseTree::Path(item) => {
            let mut path = prefix.to_vec();
            path.push(item.ident.to_string());
            collect_imports(&item.tree, &path, name, imports, globs);
        }
        UseTree::Group(item) => {
            for item in &item.items {
                collect_imports(item, prefix, name, imports, globs);
            }
        }
        UseTree::Name(item) if item.ident == name => {
            let mut path = prefix.to_vec();
            path.push(item.ident.to_string());
            imports.push(path);
        }
        UseTree::Rename(item) if item.rename == name => {
            let mut path = prefix.to_vec();
            path.push(item.ident.to_string());
            imports.push(path);
        }
        UseTree::Glob(_) => globs.push(prefix.to_vec()),
        _ => {}
    }
}

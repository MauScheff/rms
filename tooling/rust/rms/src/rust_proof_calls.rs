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
}

impl Index {
    pub(crate) fn from_parsed(files: BTreeMap<PathBuf, syn::File>, library: PathBuf, crate_name: Option<String>) -> Self {
        Self { files: files.into_iter().filter_map(|(path, file)| normalize(&path).map(|path| (path, file))).collect(), library: normalize(&library).unwrap_or(library), crate_name }
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
        }
    }

    pub(crate) fn resolve(&self, path: &Path, call: &[String]) -> Option<(PathBuf, String)> {
        self.resolve_at(path, call, 0).ok()
    }

    /// Positive reachability requires exact source identities on every edge.
    pub(crate) fn reaches(&self, start: (PathBuf, String), target: (PathBuf, String)) -> bool {
        let Some(start_path) = normalize(&start.0) else { return false; };
        let Some(target_path) = normalize(&target.0) else { return false; };
        let start = (start_path, start.1);
        let target = (target_path, target.1);
        let mut pending = vec![start];
        let mut seen = BTreeSet::new();
        while let Some((path, symbol)) = pending.pop() {
            if !seen.insert((path.clone(), symbol.clone())) { continue; }
            if seen.len() > 512 { return false; }
            let Some(file) = self.files.get(&path) else { continue; };
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
            if (path.clone(), symbol) == target { return true; }
            let (self_type, signature, body) = bodies[0].clone();
            let mut collector = Calls { index: self, path: &path, self_type, locals: BTreeMap::new(), calls: Vec::new(), local_import: false };
            for input in &signature.inputs {
                if let syn::FnArg::Typed(input) = input {
                    if let syn::Pat::Ident(name) = &*input.pat { collector.locals.insert(name.ident.to_string(), type_path(&input.ty)); }
                }
            }
            collector.visit_block(body);
            if !collector.local_import { pending.extend(collector.calls); }
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
                let root = if path.starts_with(self.library.parent().ok_or(())?) {
                    self.library.as_path()
                } else {
                    path
                };
                return self.resolve_at(root, &call[1..], depth + 1);
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
            if definitions.len() != 1 {
                return Err(());
            }
            return match definitions[0] {
                Item::Struct(_) | Item::Enum(_) if call.len() == 2 => {
                    let methods = file.items.iter().filter_map(|item| if let Item::Impl(block) = item { Some(block) } else { None })
                        .filter(|block| block.trait_.is_none() && block.attrs.is_empty() && matches!(&*block.self_ty, syn::Type::Path(ty) if ty.path.is_ident(first)))
                        .flat_map(|block| &block.items)
                        .filter(|item| matches!(item, syn::ImplItem::Fn(function) if function.sig.ident == call[1] && function.attrs.iter().all(|attr| attr.path().is_ident("doc"))))
                        .count();
                    if methods == 1 { Ok((path.to_path_buf(), call.join("::"))) } else { Err(()) }
                }
                Item::Fn(item)
                    if call.len() == 1
                        && item.attrs.iter().all(|attr| {
                            attr.path().is_ident("test") || attr.path().is_ident("doc")
                        }) =>
                {
                    Ok((path.to_path_buf(), first.clone()))
                }
                Item::Mod(item) if call.len() > 1 && item.content.is_none() => {
                    let mut explicit = Vec::new();
                    for attr in &item.attrs {
                        if let syn::Meta::NameValue(meta) = &attr.meta {
                            if meta.path.is_ident("path") {
                                if let syn::Expr::Lit(value) = &meta.value {
                                    if let syn::Lit::Str(value) = &value.lit {
                                        explicit.push(value.value());
                                        continue;
                                    }
                                }
                            }
                        }
                        if !attr.path().is_ident("doc") {
                            return Err(());
                        }
                    }
                    let parent = path.parent().ok_or(())?;
                    let candidates = match explicit.as_slice() {
                        [relative] => vec![normalize(&parent.join(relative)).ok_or(())?],
                        [] => {
                            let directory = if matches!(
                                path.file_name().and_then(|s| s.to_str()),
                                Some("lib.rs" | "main.rs" | "mod.rs")
                            ) || path
                                .parent()
                                .and_then(Path::file_name)
                                .and_then(|s| s.to_str())
                                == Some("tests")
                            {
                                parent.to_path_buf()
                            } else {
                                parent.join(path.file_stem().ok_or(())?)
                            };
                            vec![
                                directory.join(format!("{first}.rs")),
                                directory.join(first).join("mod.rs"),
                            ]
                        }
                        _ => return Err(()),
                    };
                    let existing = candidates
                        .iter()
                        .filter(|path| self.files.contains_key(*path))
                        .collect::<Vec<_>>();
                    if existing.len() != 1 {
                        return Err(());
                    }
                    self.resolve_at(existing[0], &call[1..], depth + 1)
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
        syn::Type::Path(path) if path.qself.is_none() => Some(path.path.segments.iter().map(|part| part.ident.to_string()).collect()),
        _ => None,
    }
}

struct Calls<'a> {
    index: &'a Index,
    path: &'a Path,
    self_type: Option<String>,
    locals: BTreeMap<String, Option<Vec<String>>>,
    calls: Vec<(PathBuf, String)>,
    local_import: bool,
}

impl Calls<'_> {
    fn add(&mut self, mut path: Vec<String>) {
        if path.first().is_some_and(|name| name == "Self") {
            let Some(name) = &self.self_type else { return; };
            path[0] = name.clone();
        } else if path.first().is_some_and(|name| self.locals.contains_key(name)) { return; }
        if let Some(call) = self.index.resolve(self.path, &path) { self.calls.push(call); }
    }
    fn value_type(&self, expr: &syn::Expr) -> Option<Vec<String>> {
        match expr {
            syn::Expr::Struct(value) => Some(value.path.segments.iter().map(|part| if part.ident == "Self" { self.self_type.clone().unwrap_or_default() } else { part.ident.to_string() }).collect()),
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
            if path.qself.is_none() { self.add(path.path.segments.iter().map(|part| part.ident.to_string()).collect()); }
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
        for statement in &node.stmts {
            if let syn::Stmt::Item(Item::Fn(function)) = statement { self.locals.insert(function.sig.ident.to_string(), None); }
        }
        visit::visit_block(self, node);
        self.locals = saved;
    }
    fn visit_item(&mut self, node: &'ast Item) {
        // Local functions/imports are not executed by being declared.
        if let Item::Fn(function) = node { self.locals.insert(function.sig.ident.to_string(), None); }
        if matches!(node, Item::Use(_)) { self.local_import = true; }
    }
    fn visit_expr_closure(&mut self, _node: &'ast syn::ExprClosure) {}
}

#[cfg(test)]
mod tests {
    use super::*;

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

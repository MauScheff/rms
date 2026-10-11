//! Source-owned lexical imports for dependency checks, not purity authority.
//! Unknown wildcard exports never establish a binding. External aliases retain
//! their original crate identity, including when publicly re-exported.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use syn::{Item, UseTree, Visibility};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Binding {
    Module(String),
    Item(String, String),
    External(Vec<String>),
}

#[derive(Clone)]
struct Import {
    path: Vec<String>,
    alias: Option<String>,
    public: bool,
    absolute: bool,
}

#[derive(Default)]
struct Scope {
    file: PathBuf,
    root: String,
    parent: Option<String>,
    declarations: BTreeMap<String, Binding>,
    public: BTreeSet<String>,
    imports: Vec<Import>,
    names: BTreeMap<String, BTreeSet<Binding>>,
    exports: BTreeMap<String, BTreeSet<Binding>>,
    opaque_exports: bool,
}

pub(super) struct Resolver {
    scopes: BTreeMap<String, Scope>,
    crate_roots: BTreeMap<String, String>,
    members: BTreeMap<(String, String), BTreeMap<String, BTreeSet<Binding>>>,
    value_items: BTreeSet<(String, String)>,
}

fn normalized(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                result.pop();
            }
            _ => result.push(part.as_os_str()),
        }
    }
    result
}

fn imports(
    tree: &UseTree,
    prefix: Vec<String>,
    public: bool,
    absolute: bool,
    output: &mut Vec<Import>,
) {
    match tree {
        UseTree::Path(item) => {
            let mut prefix = prefix;
            prefix.push(item.ident.to_string());
            imports(&item.tree, prefix, public, absolute, output);
        }
        UseTree::Group(group) => {
            for item in &group.items {
                imports(item, prefix.clone(), public, absolute, output);
            }
        }
        UseTree::Glob(_) => output.push(Import {
            path: prefix,
            alias: None,
            public,
            absolute,
        }),
        UseTree::Name(item) => {
            let name = item.ident.to_string();
            let mut path = prefix;
            if name != "self" || path.is_empty() {
                path.push(name.clone());
            }
            let alias = path.last().cloned();
            output.push(Import {
                path,
                alias,
                public,
                absolute,
            });
        }
        UseTree::Rename(item) => {
            let mut path = prefix;
            if item.ident != "self" || path.is_empty() {
                path.push(item.ident.to_string());
            }
            output.push(Import {
                path,
                alias: Some(item.rename.to_string()),
                public,
                absolute,
            });
        }
    }
}

impl Resolver {
    pub(super) fn new(
        files: &BTreeMap<PathBuf, syn::File>,
        library: &Path,
        crates: &BTreeSet<String>,
    ) -> Self {
        let files = files
            .iter()
            .map(|(path, file)| (normalized(path), file))
            .collect::<BTreeMap<_, _>>();
        let mut result = Self {
            scopes: BTreeMap::new(),
            crate_roots: BTreeMap::new(),
            members: BTreeMap::new(),
            value_items: BTreeSet::new(),
        };
        let library = normalized(library);
        if let Some(file) = files.get(&library) {
            let id = library.display().to_string();
            result.add_scope(
                &files,
                &library,
                &id,
                &id,
                None,
                &file.items,
                library.parent().unwrap().to_path_buf(),
            );
            for name in crates {
                result.crate_roots.insert(name.clone(), id.clone());
            }
        }
        for (path, file) in &files {
            if !result.scopes.values().any(|scope| scope.file == *path) {
                let id = path.display().to_string();
                result.add_scope(
                    &files,
                    path,
                    &id,
                    &id,
                    None,
                    &file.items,
                    path.parent().unwrap().to_path_buf(),
                );
            }
        }
        // Imports are module items: textual ordering must not decide resolution.
        // The graph is finite. A non-convergent alias graph supplies no lexical
        // proof; never let malformed source hang the validator or router.
        let budget = result
            .scopes
            .values()
            .map(|scope| scope.imports.len() + scope.declarations.len() + 1)
            .sum::<usize>()
            * 2
            + 1;
        for iteration in 0..budget {
            let mut next = BTreeMap::new();
            for (id, scope) in &result.scopes {
                let mut explicit = scope
                    .declarations
                    .iter()
                    .map(|(name, binding)| (name.clone(), BTreeSet::from([binding.clone()])))
                    .collect::<BTreeMap<_, _>>();
                let mut globs: BTreeMap<String, BTreeSet<Binding>> = BTreeMap::new();
                let mut exports = scope
                    .declarations
                    .iter()
                    .filter(|(name, _)| scope.public.contains(*name))
                    .map(|(name, binding)| (name.clone(), BTreeSet::from([binding.clone()])))
                    .collect::<BTreeMap<_, _>>();
                let mut public_globs: BTreeMap<String, BTreeSet<Binding>> = BTreeMap::new();
                for import in &scope.imports {
                    if let Some(binding) = result.resolve(id, import) {
                        if let Some(alias) = &import.alias {
                            if alias != "_" {
                                explicit
                                    .entry(alias.clone())
                                    .or_default()
                                    .insert(binding.clone());
                                if import.public {
                                    exports.entry(alias.clone()).or_default().insert(binding);
                                }
                            }
                        } else {
                            for (name, bindings) in result.visible_glob(id, &binding, import.public)
                            {
                                globs
                                    .entry(name.clone())
                                    .or_default()
                                    .extend(bindings.clone());
                                if import.public {
                                    public_globs.entry(name).or_default().extend(bindings);
                                }
                            }
                        }
                    }
                }
                for (name, bindings) in public_globs {
                    if !explicit.contains_key(&name) {
                        exports.entry(name).or_default().extend(bindings);
                    }
                }
                for (name, bindings) in globs {
                    explicit.entry(name).or_insert(bindings);
                }
                next.insert(id.clone(), (explicit, exports));
            }
            if next.iter().all(|(id, (names, exports))| {
                result.scopes[id].names == *names && result.scopes[id].exports == *exports
            }) {
                break;
            }
            for (id, (names, exports)) in next {
                let scope = result.scopes.get_mut(&id).unwrap();
                scope.names = names;
                scope.exports = exports;
            }
            if iteration + 1 == budget {
                for scope in result.scopes.values_mut() {
                    scope.names = scope
                        .declarations
                        .iter()
                        .map(|(name, binding)| (name.clone(), BTreeSet::from([binding.clone()])))
                        .collect();
                    scope.exports.clear();
                }
            }
        }
        result
    }

    fn add_scope(
        &mut self,
        files: &BTreeMap<PathBuf, &syn::File>,
        path: &Path,
        id: &str,
        root: &str,
        parent: Option<String>,
        items: &[Item],
        module_dir: PathBuf,
    ) {
        if self.scopes.contains_key(id) {
            return;
        }
        let mut scope = Scope {
            file: path.to_path_buf(),
            root: root.into(),
            parent,
            ..Scope::default()
        };
        // Reserve the identity before recursive module discovery.
        self.scopes.insert(
            id.into(),
            Scope {
                file: path.to_path_buf(),
                root: root.into(),
                parent: scope.parent.clone(),
                ..Scope::default()
            },
        );
        for item in items {
            if matches!(item, Item::Macro(_)) {
                scope.opaque_exports = true;
            }
            let value_name = match item {
                Item::Fn(item) => Some(item.sig.ident.to_string()),
                Item::Const(item) => Some(item.ident.to_string()),
                Item::Static(item) => Some(item.ident.to_string()),
                _ => None,
            };
            if let Some(name) = value_name {
                self.value_items.insert((id.into(), name));
            }
            let declaration = match item {
                Item::Struct(item) => Some((item.ident.to_string(), &item.vis)),
                Item::Enum(item) => {
                    let name = item.ident.to_string();
                    self.members.insert(
                        (id.into(), name.clone()),
                        item.variants
                            .iter()
                            .map(|variant| {
                                let variant = variant.ident.to_string();
                                (
                                    variant.clone(),
                                    BTreeSet::from([Binding::Item(
                                        id.into(),
                                        format!("{name}::{variant}"),
                                    )]),
                                )
                            })
                            .collect(),
                    );
                    Some((name, &item.vis))
                }
                Item::Type(item) => Some((item.ident.to_string(), &item.vis)),
                Item::Trait(item) => Some((item.ident.to_string(), &item.vis)),
                Item::Union(item) => Some((item.ident.to_string(), &item.vis)),
                Item::Fn(item) => Some((item.sig.ident.to_string(), &item.vis)),
                Item::Const(item) => Some((item.ident.to_string(), &item.vis)),
                Item::Static(item) => Some((item.ident.to_string(), &item.vis)),
                Item::Mod(item) => {
                    let name = item.ident.to_string();
                    let child = format!("{id}::{name}");
                    let mut child_path = module_dir.join(format!("{name}.rs"));
                    for attr in &item.attrs {
                        if attr.path().is_ident("path") {
                            if let syn::Meta::NameValue(value) = &attr.meta {
                                if let syn::Expr::Lit(literal) = &value.value {
                                    if let syn::Lit::Str(value) = &literal.lit {
                                        child_path = module_dir.join(value.value());
                                    }
                                }
                            }
                        }
                    }
                    child_path = normalized(&child_path);
                    if let Some((_, items)) = &item.content {
                        self.add_scope(
                            files,
                            path,
                            &child,
                            root,
                            Some(id.into()),
                            items,
                            module_dir.join(&name),
                        );
                    } else {
                        if !files.contains_key(&child_path) {
                            child_path = module_dir.join(&name).join("mod.rs");
                        }
                        if let Some(file) = files
                            .get(&child_path)
                            .filter(|_| !self.ancestral_file(id, &child_path))
                        {
                            let directory =
                                if child_path.file_name().is_some_and(|name| name == "mod.rs") {
                                    child_path.parent().unwrap().to_path_buf()
                                } else {
                                    child_path.with_extension("")
                                };
                            self.add_scope(
                                files,
                                &child_path,
                                &child,
                                root,
                                Some(id.into()),
                                &file.items,
                                directory,
                            );
                        }
                    }
                    scope
                        .declarations
                        .insert(name.clone(), Binding::Module(child));
                    if matches!(item.vis, Visibility::Public(_)) {
                        scope.public.insert(name);
                    }
                    None
                }
                Item::Use(item) => {
                    imports(
                        &item.tree,
                        Vec::new(),
                        matches!(item.vis, Visibility::Public(_)),
                        item.leading_colon.is_some(),
                        &mut scope.imports,
                    );
                    None
                }
                Item::ExternCrate(item) => {
                    scope.imports.push(Import {
                        path: vec![item.ident.to_string()],
                        alias: Some(
                            item.rename.as_ref().map_or_else(
                                || item.ident.to_string(),
                                |(_, name)| name.to_string(),
                            ),
                        ),
                        public: matches!(item.vis, Visibility::Public(_)),
                        absolute: true,
                    });
                    None
                }
                _ => None,
            };
            if let Some((name, visibility)) = declaration {
                scope
                    .declarations
                    .insert(name.clone(), Binding::Item(id.into(), name.clone()));
                if matches!(visibility, Visibility::Public(_)) {
                    scope.public.insert(name);
                }
            }
        }
        scope.names = scope
            .declarations
            .iter()
            .map(|(name, binding)| (name.clone(), BTreeSet::from([binding.clone()])))
            .collect();
        self.scopes.insert(id.into(), scope);
    }

    fn exported(&self, binding: &Binding) -> BTreeMap<String, BTreeSet<Binding>> {
        match binding {
            Binding::Module(id) => self
                .scopes
                .get(id)
                .map(|scope| scope.exports.clone())
                .unwrap_or_default(),
            Binding::Item(id, name) => self
                .members
                .get(&(id.clone(), name.clone()))
                .cloned()
                .unwrap_or_default(),
            Binding::External(_) => BTreeMap::new(),
        }
    }

    fn ancestral_file(&self, id: &str, path: &Path) -> bool {
        let mut current = Some(id);
        while let Some(id) = current {
            let Some(scope) = self.scopes.get(id) else {
                break;
            };
            if scope.file == path {
                return true;
            }
            current = scope.parent.as_deref();
        }
        false
    }

    fn visible_glob(
        &self,
        requester: &str,
        binding: &Binding,
        public: bool,
    ) -> BTreeMap<String, BTreeSet<Binding>> {
        if !public {
            if let Binding::Module(target) = binding {
                let mut current = Some(requester);
                while let Some(id) = current {
                    let Some(scope) = self.scopes.get(id) else {
                        break;
                    };
                    // A descendant can see its ancestor's private bindings.
                    // This does not publish those bindings through a crate glob.
                    if id == target {
                        return scope.names.clone();
                    }
                    current = scope.parent.as_deref();
                }
            }
        }
        self.exported(binding)
    }

    fn opaque_glob(&self, binding: &Binding, visiting: &mut BTreeSet<String>) -> bool {
        match binding {
            Binding::External(_) => true,
            Binding::Item(..) => false,
            Binding::Module(id) => {
                if !visiting.insert(id.clone()) {
                    return true;
                }
                let opaque = self.scopes.get(id).is_none_or(|scope| {
                    scope.opaque_exports
                        || scope
                            .imports
                            .iter()
                            .filter(|import| import.public)
                            .any(|import| {
                                self.resolve(id, import).is_none_or(|binding| {
                                    import.alias.is_none() && self.opaque_glob(&binding, visiting)
                                })
                            })
                });
                visiting.remove(id);
                opaque
            }
        }
    }

    fn select(&self, names: &BTreeSet<Binding>) -> Option<Binding> {
        if names.len() == 1 {
            return names.first().cloned();
        }
        // A module inhabits the type namespace. Functions/constants inhabit the
        // value namespace. A valid module/function pair is not an ambiguous glob.
        let modules = names
            .iter()
            .filter(|binding| matches!(binding, Binding::Module(_)))
            .collect::<Vec<_>>();
        if modules.len() == 1
            && names.iter().all(|binding| match binding {
                Binding::Module(_) => true,
                Binding::Item(id, name) => self.value_items.contains(&(id.clone(), name.clone())),
                Binding::External(_) => false,
            })
        {
            return Some(modules[0].clone());
        }
        None
    }

    fn resolve(&self, id: &str, import: &Import) -> Option<Binding> {
        let scope = &self.scopes[id];
        let first = import.path.first()?;
        let mut binding = if !import.absolute {
            match first.as_str() {
                "crate" => Binding::Module(scope.root.clone()),
                "self" => Binding::Module(id.into()),
                "super" => Binding::Module(scope.parent.clone()?),
                _ => {
                    if import.alias.as_ref() == Some(first)
                        && !scope.declarations.contains_key(first)
                    {
                        // An import cannot resolve through its own alias. Inspect
                        // known globs before using the external prelude instead.
                        let mut matches = BTreeSet::new();
                        for glob in scope
                            .imports
                            .iter()
                            .filter(|glob| glob.alias.is_none() && glob.path.first() != Some(first))
                        {
                            if let Some(binding) = self.resolve(id, glob) {
                                if let Some(names) =
                                    self.visible_glob(id, &binding, glob.public).get(first)
                                {
                                    matches.extend(names.iter().cloned());
                                }
                            }
                        }
                        if !matches.is_empty() {
                            self.select(&matches)?
                        } else if let Some(root) = self.crate_roots.get(first) {
                            Binding::Module(root.clone())
                        } else {
                            Binding::External(vec![first.clone()])
                        }
                    } else if let Some(names) = scope.names.get(first) {
                        self.select(names)?
                    } else if scope.imports.iter().any(|declared| {
                        declared.alias.as_ref() == Some(first) && !std::ptr::eq(declared, import)
                    }) {
                        // An unresolved lexical alias is not an external-prelude
                        // root. This also makes alias cycles converge without guesses.
                        return None;
                    } else if let Some(root) = self.crate_roots.get(first) {
                        Binding::Module(root.clone())
                    } else {
                        Binding::External(vec![first.clone()])
                    }
                }
            }
        } else if let Some(root) = self.crate_roots.get(first) {
            Binding::Module(root.clone())
        } else {
            Binding::External(vec![first.clone()])
        };
        for part in import.path.iter().skip(1) {
            binding = match binding {
                Binding::External(mut path) => {
                    path.push(part.clone());
                    Binding::External(path)
                }
                Binding::Module(ref module) if part == "super" => {
                    Binding::Module(self.scopes.get(module)?.parent.clone()?)
                }
                Binding::Module(ref module) => {
                    let names = self.scopes.get(module)?.names.get(part)?;
                    self.select(names)?
                }
                ref item => {
                    let exports = self.exported(item);
                    let names = exports.get(part)?;
                    self.select(names)?
                }
            };
        }
        if import.alias.is_none() {
            match &binding {
                Binding::Item(id, name)
                    if !self.members.contains_key(&(id.clone(), name.clone())) =>
                {
                    return None
                }
                Binding::Module(id) if !self.scopes.contains_key(id) => return None,
                _ => {}
            }
        }
        Some(binding)
    }

    pub(super) fn imports(
        &self,
        path: &Path,
        local_modules: &BTreeSet<String>,
    ) -> Vec<(super::RustImport, super::RustImportRootKind)> {
        let path = normalized(path);
        let mut output = Vec::new();
        for (id, scope) in &self.scopes {
            if scope.file != path {
                continue;
            }
            for import in &scope.imports {
                let Some(first) = import.path.first() else {
                    continue;
                };
                let (root, kind) = match self.resolve(id, import) {
                    Some(Binding::External(path)) => {
                        let root = path[0].clone();
                        let opaque_scope = !import.absolute
                            && !matches!(root.as_str(), "std" | "core" | "alloc")
                            && root == *first
                            && !scope.names.contains_key(first)
                            && scope
                                .imports
                                .iter()
                                .filter(|glob| glob.alias.is_none() && !std::ptr::eq(*glob, import))
                                .any(|glob| {
                                    self.resolve(id, glob).is_none_or(|binding| {
                                        self.opaque_glob(&binding, &mut BTreeSet::new())
                                    })
                                });
                        let kind = if opaque_scope {
                            super::RustImportRootKind::Unresolved
                        } else {
                            super::rust_import_root_kind(
                                &root,
                                &self.crate_roots.keys().cloned().collect(),
                            )
                        };
                        (root, kind)
                    }
                    Some(_) if matches!(first.as_str(), "crate" | "self" | "super") => {
                        (first.clone(), super::RustImportRootKind::SelfQualified)
                    }
                    Some(_) if local_modules.contains(first) => {
                        (first.clone(), super::RustImportRootKind::LocalModule)
                    }
                    Some(_) => (first.clone(), super::RustImportRootKind::SelfQualified),
                    None => (first.clone(), super::RustImportRootKind::Unresolved),
                };
                let resolved = (
                    super::RustImport {
                        root,
                        is_public: import.public,
                    },
                    kind,
                );
                if !output.contains(&resolved) {
                    output.push(resolved);
                }
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(
        library: &str,
        consumer: &str,
    ) -> Vec<(super::super::RustImport, super::super::RustImportRootKind)> {
        let files = BTreeMap::from([
            (
                PathBuf::from("/fixture/src/lib.rs"),
                syn::parse_file(library).unwrap(),
            ),
            (
                PathBuf::from("/fixture/tests/consumer.rs"),
                syn::parse_file(consumer).unwrap(),
            ),
        ]);
        Resolver::new(
            &files,
            Path::new("/fixture/src/lib.rs"),
            &BTreeSet::from(["provider".into()]),
        )
        .imports(
            Path::new("/fixture/tests/consumer.rs"),
            &BTreeSet::from(["provider".into()]),
        )
    }

    #[test]
    fn lexical_aliases_keep_source_and_external_crate_identity() {
        use super::super::RustImportRootKind::{External, SelfQualified, Standard, Unresolved};
        let library = "mod internal { pub enum Command { Start } pub struct Input; } pub use crate::internal::*;";
        let same_namespaces = "mod transition { pub enum Command { Start } pub fn transition() {} } pub use crate::transition::{Command, transition};";
        assert!(
            !resolve(same_namespaces, "use provider::*; use Command as C;")
                .iter()
                .any(|(_, kind)| matches!(kind, External | Unresolved))
        );
        let private_module = resolve(same_namespaces, "use provider::*; use transition::*;");
        assert!(
            private_module
                .iter()
                .any(|(import, kind)| import.root == "transition" && *kind == Unresolved),
            "a public function must not publish its same-named private module"
        );
        for consumer in [
            "use provider::*; use Command as C; use Input as I; use C as Again;",
            "use C as Again; use Command as C; use provider::*;",
            "use provider as renamed; use renamed::*; use Command as C;",
            "use provider::*; use Command as Command; use Command::*; use Start as S;",
            "mod nested { use provider::*; use Input as I; }",
            "struct Local; use Local as L; type Alias = Local; use Alias as A;",
            "enum Private { Ready } mod nested { use super::*; use Private as P; }",
        ] {
            assert!(
                !resolve(library, consumer)
                    .iter()
                    .any(|(_, kind)| *kind == External),
                "{consumer}"
            );
        }
        let aliased = resolve(library, "use undeclared::Type; pub use Type as Public; use undeclared as another; use another::Other;");
        assert!(aliased
            .iter()
            .all(|(import, kind)| import.root == "undeclared" && *kind == External));
        assert!(aliased.iter().any(|(import, _)| import.is_public));
        assert!(
            resolve(library, "use std::fs as std; use std::read as read;")
                .iter()
                .all(|(_, kind)| *kind == Standard)
        );
        assert!(resolve(library, "use provider::*; use MissingType as M;")
            .iter()
            .any(|(import, kind)| import.root == "MissingType" && *kind == External));
        assert!(
            resolve(library, "struct undeclared; use ::undeclared::Thing;")
                .iter()
                .any(|(import, kind)| import.root == "undeclared" && *kind == External)
        );
        let ambiguous = resolve(
            "pub mod a { pub struct Same; } pub mod b { pub struct Same; }",
            "use provider::a::*; use provider::b::*; use Same as S;",
        );
        assert!(ambiguous
            .iter()
            .any(|(import, kind)| import.root == "Same" && *kind == Unresolved));
        assert!(resolve(
            library,
            "use provider::*; use provider::Command as C; pub use C as Public;"
        )
        .iter()
        .any(|(import, kind)| import.is_public && *kind == SelfQualified));
        // Unseeded lexical and public re-export cycles remain unresolved.
        assert!(resolve(library, "use A as B; use B as A;")
            .iter()
            .all(|(_, kind)| *kind == Unresolved));
        let cycle =
            "mod a { pub use crate::b::T; } mod b { pub use crate::a::T; } pub use crate::a::*;";
        assert!(resolve(cycle, "use provider::*; use T as Alias;")
            .iter()
            .any(|(import, kind)| import.root == "T" && *kind == Unresolved));
        assert!(resolve(library, "use opaque::*; use Maybe as Alias;")
            .iter()
            .any(|(import, kind)| import.root == "Maybe" && *kind == Unresolved));
    }
}

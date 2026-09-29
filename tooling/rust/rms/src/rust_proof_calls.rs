//! Exact, source-local call identity for trace proof inspection.
//! This index does not infer effects or trust names from unrelated source files.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};
use syn::{Item, UseTree};

pub(crate) struct Index {
    files: BTreeMap<PathBuf, syn::File>,
    library: PathBuf,
    crate_name: Option<String>,
}

impl Index {
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
                _ => false,
            })
            .collect::<Vec<_>>();
        if !definitions.is_empty() {
            if definitions.len() != 1 {
                return Err(());
            }
            return match definitions[0] {
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

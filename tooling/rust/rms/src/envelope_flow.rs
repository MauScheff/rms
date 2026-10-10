//! Closed, source-owned envelope projection proof. No IO or scheduling authority.
use super::{EnvelopeFlow, ExecutorBinding, FlowContract, Index, Types};
use proc_macro2::{TokenStream, TokenTree};
use std::collections::{BTreeMap, BTreeSet};
use syn::{
    ext::IdentExt,
    parse::Parser,
    spanned::Spanned,
    visit::{self, Visit},
    Expr, FnArg, Item, Pat, ReturnType, Stmt, Type,
};

type Proof<T> = Result<T, String>;
fn need(value: bool, reason: &str) -> Proof<()> {
    if value {
        Ok(())
    } else {
        Err(reason.into())
    }
}
fn address<T>(value: &T) -> usize {
    value as *const T as usize
}

// This exception is derived from syntax, never from a module name or path.
// No runtime source owner can be reached through this declaration.
fn test_only_module(item: &syn::ItemMod) -> bool {
    let mut conditions = 0;
    let mut paths = 0;
    for attr in &item.attrs {
        if attr.path().is_ident("doc") {
            continue;
        }
        if attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Path>()
                .is_ok_and(|path| path.is_ident("test"))
        {
            conditions += 1;
        } else if attr.path().is_ident("path") {
            let syn::Meta::NameValue(value) = &attr.meta else {
                return false;
            };
            let Expr::Lit(value) = &value.value else {
                return false;
            };
            let syn::Lit::Str(value) = &value.lit else {
                return false;
            };
            if value.value().is_empty() {
                return false;
            }
            paths += 1;
        } else {
            return false;
        }
    }
    conditions == 1 && paths <= 1
}

// Only replacement fields carry implicit references. Prose and escaped braces
// remain data. Unknown field/specification grammar is refused, never skipped.
fn format_capture_names(text: &str) -> Proof<Vec<(String, usize)>> {
    fn argument(text: &str, names: &mut Vec<(String, usize)>, offset: usize) -> Proof<()> {
        if text.is_empty() || text.bytes().all(|byte| byte.is_ascii_digit()) {
            return Ok(());
        }
        need(
            text != "_" && syn::Ident::parse_any.parse_str(text).is_ok(),
            "unsupported format argument",
        )?;
        names.push((text.into(), offset));
        Ok(())
    }
    fn count(spec: &mut &str, names: &mut Vec<(String, usize)>, offset: usize) -> Proof<bool> {
        let digits = spec.bytes().take_while(u8::is_ascii_digit).count();
        if digits > 0 {
            *spec = &spec[digits..];
            if spec.starts_with('$') {
                *spec = &spec[1..];
            }
            return Ok(true);
        }
        if !spec.starts_with('.') {
            if let Some(end) = spec.find('$') {
                argument(&spec[..end], names, offset)?;
                need(end > 0, "empty format count")?;
                *spec = &spec[end + 1..];
                return Ok(true);
            }
        }
        Ok(false)
    }
    let mut names = Vec::new();
    let mut cursor = 0;
    while cursor < text.len() {
        let rest = &text[cursor..];
        if rest.starts_with("{{") || rest.starts_with("}}") {
            cursor += 2;
            continue;
        }
        if rest.starts_with('}') {
            return Err("unmatched format closing brace".into());
        }
        if !rest.starts_with('{') {
            cursor += rest.chars().next().unwrap().len_utf8();
            continue;
        }
        let end = rest.find('}').ok_or("unclosed format replacement field")?;
        let field = rest[1..end].trim_end();
        need(!field.contains('{'), "nested format replacement field")?;
        let (arg, mut spec) = field.split_once(':').unwrap_or((field, ""));
        argument(arg, &mut names, cursor)?;
        let mut chars = spec.char_indices();
        let first = chars.next();
        let second = chars.next();
        if second.is_some_and(|(_, ch)| matches!(ch, '<' | '^' | '>')) {
            let (index, ch) = second.unwrap();
            spec = &spec[index + ch.len_utf8()..];
        } else if first.is_some_and(|(_, ch)| matches!(ch, '<' | '^' | '>')) {
            spec = &spec[1..];
        }
        if spec.starts_with(['+', '-']) {
            spec = &spec[1..];
        }
        if spec.starts_with('#') {
            spec = &spec[1..];
        }
        if spec.starts_with('0') && !spec.starts_with("0$") {
            spec = &spec[1..];
        }
        count(&mut spec, &mut names, cursor)?;
        if spec.starts_with('.') {
            spec = &spec[1..];
            if spec.starts_with('*') {
                spec = &spec[1..];
            } else {
                need(
                    count(&mut spec, &mut names, cursor)?,
                    "missing format precision",
                )?;
            }
        }
        need(
            matches!(
                spec,
                "" | "?" | "x?" | "X?" | "o" | "x" | "X" | "p" | "b" | "e" | "E"
            ),
            "unsupported format specification",
        )?;
        cursor += end + 1;
    }
    Ok(names)
}
fn plain(path: &syn::Path) -> Option<String> {
    (path.leading_colon.is_none()
        && path
            .segments
            .iter()
            .all(|part| matches!(part.arguments, syn::PathArguments::None)))
    .then(|| {
        path.segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect::<Vec<_>>()
            .join("::")
    })
}
fn import_spelling(tree: &syn::UseTree) -> String {
    match tree {
        syn::UseTree::Path(value) => format!("{}::{}", value.ident, import_spelling(&value.tree)),
        syn::UseTree::Name(value) => value.ident.to_string(),
        syn::UseTree::Rename(value) => format!("{} as {}", value.ident, value.rename),
        syn::UseTree::Glob(_) => "*".into(),
        syn::UseTree::Group(value) => format!(
            "{{{}}}",
            value
                .items
                .iter()
                .map(import_spelling)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}
fn atom(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::Path(path) if path.qself.is_none() && path.path.is_ident(name))
}
fn identifier(pattern: &Pat) -> Option<String> {
    match pattern {
        Pat::Ident(value)
            if value.by_ref.is_none() && value.mutability.is_none() && value.subpat.is_none() =>
        {
            Some(value.ident.to_string())
        }
        _ => None,
    }
}
fn argument(arg: &FnArg) -> Proof<(String, &Type)> {
    match arg {
        FnArg::Typed(value) => identifier(&value.pat)
            .map(|name| (name, value.ty.as_ref()))
            .ok_or("parameters must be immutable identifiers".into()),
        _ => Err("an owned typed parameter is required".into()),
    }
}
fn tail(block: &syn::Block) -> Proof<&Expr> {
    match block.stmts.as_slice() {
        [Stmt::Expr(expr, None)] => Ok(expr),
        _ => Err("unsupported body: require one tail expression".into()),
    }
}
fn projection(expr: &Expr, owner: &str, field: &str) -> bool {
    matches!(expr, Expr::Field(value) if atom(&value.base, owner) && matches!(&value.member, syn::Member::Named(name) if name == field))
}
fn fields(item: &syn::Fields) -> Proof<BTreeMap<String, &Type>> {
    let syn::Fields::Named(named) = item else {
        return Err("named fields are required".into());
    };
    let mut result = BTreeMap::new();
    for field in &named.named {
        need(
            field.attrs.iter().all(|attr| attr.path().is_ident("doc")),
            "conditional or transformed fields are unsupported",
        )?;
        let name = field.ident.as_ref().ok_or("unnamed field")?.to_string();
        need(result.insert(name, &field.ty).is_none(), "duplicate field")?;
    }
    Ok(result)
}
fn literal_fields(expr: &syn::ExprStruct) -> Proof<BTreeMap<String, &Expr>> {
    need(
        expr.qself.is_none() && expr.rest.is_none(),
        "struct update and qualified-self constructors are unsupported",
    )?;
    let mut result = BTreeMap::new();
    for field in &expr.fields {
        let syn::Member::Named(name) = &field.member else {
            return Err("unnamed construction field".into());
        };
        need(
            field.attrs.is_empty() && result.insert(name.to_string(), &field.expr).is_none(),
            "conditional or duplicate construction field",
        )?;
    }
    Ok(result)
}
fn module(path: &str) -> Proof<String> {
    let path = path
        .strip_prefix("src/")
        .and_then(|path| path.strip_suffix(".rs"))
        .ok_or("flow sources must be ordinary src/*.rs modules")?;
    need(
        !path.split('/').any(|part| matches!(part, "" | "." | "..")),
        "unsupported module path",
    )?;
    Ok(if matches!(path, "lib" | "main") {
        "crate".into()
    } else {
        format!(
            "crate::{}",
            path.trim_end_matches("/mod").replace('/', "::")
        )
    })
}

struct Callable<'a> {
    file: &'a str,
    sig: &'a syn::Signature,
    body: &'a syn::Block,
    vis: &'a syn::Visibility,
}
struct Checker<'a> {
    index: &'a Index<'a>,
    protected: BTreeSet<String>,
    callable_leaves: BTreeSet<String>,
    allowed_paths: BTreeSet<usize>,
    allowed_types: BTreeSet<usize>,
    allowed_structs: BTreeSet<usize>,
    allowed_methods: BTreeSet<usize>,
    context_impl: usize,
}
impl<'a> Checker<'a> {
    fn modules(&self, required: &BTreeSet<&str>) -> Proof<BTreeSet<String>> {
        let roots = ["src/lib.rs", "src/main.rs"]
            .into_iter()
            .filter(|root| self.index.files.contains_key(*root))
            .collect::<Vec<_>>();
        need(
            roots.len() == 1,
            "flow requires one inspected ordinary Rust crate root",
        )?;
        let mut pending = vec![roots[0].to_string()];
        let mut reached = BTreeSet::new();
        let mut test_sources = BTreeSet::new();
        while let Some(file) = pending.pop() {
            need(
                reached.insert(file.clone()),
                "duplicate source module wiring",
            )?;
            let mut names = BTreeSet::new();
            for item in &self.index.files[&file].items {
                if let Item::Mod(item) = item {
                    need(
                        names.insert(item.ident.to_string()),
                        "duplicate module declaration",
                    )?;
                    if test_only_module(item) {
                        if item.content.is_none() {
                            if let Some(attr) = item.attrs.iter().find(|attr| attr.path().is_ident("path")) {
                                let syn::Meta::NameValue(value) = &attr.meta else { unreachable!() };
                                let Expr::Lit(value) = &value.value else { unreachable!() };
                                let syn::Lit::Str(value) = &value.lit else { unreachable!() };
                                let parent = std::path::Path::new(&file).parent().unwrap();
                                let joined = parent.join(value.value());
                                let mut parts = Vec::new();
                                for component in joined.components() {
                                    match component {
                                        std::path::Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
                                        std::path::Component::CurDir => {},
                                        std::path::Component::ParentDir if !parts.is_empty() => { parts.pop(); },
                                        _ => return Err("test module path escapes the inspected relative source scope".into()),
                                    }
                                }
                                test_sources.insert(parts.join("/"));
                            } else {
                                let stem = file.strip_suffix(".rs").unwrap();
                                let directory = if matches!(file.as_str(), "src/lib.rs" | "src/main.rs") {
                                    "src"
                                } else {
                                    stem.strip_suffix("/mod").unwrap_or(stem)
                                };
                                test_sources.insert(format!("{directory}/{}.rs", item.ident));
                                test_sources.insert(format!("{directory}/{}/mod.rs", item.ident));
                            }
                        }
                        continue;
                    }
                    if item.content.is_some() {
                        continue;
                    }
                    need(
                        item.attrs.iter().all(|attr| attr.path().is_ident("doc")),
                        "conditional or path-remapped source modules are unsupported",
                    )?;
                    let stem = file
                        .strip_suffix(".rs")
                        .ok_or("unsupported source extension")?;
                    let directory = if matches!(file.as_str(), "src/lib.rs" | "src/main.rs") {
                        "src"
                    } else if let Some(directory) = stem.strip_suffix("/mod") {
                        directory
                    } else {
                        stem
                    };
                    let candidates = [
                        format!("{directory}/{}.rs", item.ident),
                        format!("{directory}/{}/mod.rs", item.ident),
                    ]
                    .into_iter()
                    .filter(|path| self.index.files.contains_key(path))
                    .collect::<Vec<_>>();
                    need(
                        candidates.len() == 1,
                        "source module is missing, ambiguous or outside the inspected closure",
                    )?;
                    pending.push(candidates[0].clone());
                }
            }
            // An imported module alias must not replace a proved source owner.
            for item in &self.index.files[&file].items {
                if let Item::Use(item) = item {
                    fn bound(tree: &syn::UseTree, names: &mut Vec<String>) {
                        match tree {
                            syn::UseTree::Path(value) => bound(&value.tree, names),
                            syn::UseTree::Name(value) => names.push(value.ident.to_string()),
                            syn::UseTree::Rename(value) => names.push(value.rename.to_string()),
                            syn::UseTree::Group(value) => {
                                for value in &value.items {
                                    bound(value, names);
                                }
                            }
                            _ => {}
                        }
                    }
                    let mut imported = Vec::new();
                    bound(&item.tree, &mut imported);
                    need(
                        imported.iter().all(|name| !names.contains(name)),
                        "import shadows an inspected source module",
                    )?;
                }
            }
        }
        need(
            required.iter().all(|file| reached.contains(*file)),
            "registered source is not wired through the inspected crate root",
        )?;
        // A source reached through production wiring remains production even
        // when another declaration also uses it for tests.
        Ok(test_sources.difference(&reached).cloned().collect())
    }
    fn imports(&self, file: &str) -> Proof<BTreeMap<String, String>> {
        fn collect(
            tree: &syn::UseTree,
            prefix: &str,
            map: &mut BTreeMap<String, String>,
        ) -> Proof<()> {
            match tree {
                syn::UseTree::Path(value) => {
                    collect(&value.tree, &format!("{prefix}{}::", value.ident), map)
                }
                syn::UseTree::Name(value) => need(
                    map.insert(value.ident.to_string(), format!("{prefix}{}", value.ident))
                        .is_none(),
                    "ambiguous import",
                ),
                syn::UseTree::Group(value) => {
                    for value in &value.items {
                        collect(value, prefix, map)?;
                    }
                    Ok(())
                }
                _ => Err("flow-participating files require explicit, unaliased imports".into()),
            }
        }
        let mut imports = BTreeMap::new();
        for item in &self
            .index
            .files
            .get(file)
            .ok_or("source file missing")?
            .items
        {
            if let Item::Use(item) = item {
                let scope = format!(
                    "{file}: file-level import `{}`",
                    import_spelling(&item.tree)
                );
                if let Some(attr) = item.attrs.iter().find(|attr| !attr.path().is_ident("doc")) {
                    let name = attr
                        .path()
                        .segments
                        .iter()
                        .map(|part| part.ident.to_string())
                        .collect::<Vec<_>>()
                        .join("::");
                    return Err(format!("{scope}: unsupported import attribute `{name}`; only built-in documentation attributes are supported"));
                }
                collect(&item.tree, "", &mut imports)
                    .map_err(|reason| format!("{scope}: {reason}"))?;
            }
        }
        Ok(imports)
    }
    fn reference(&self, file: &str, path: &syn::Path) -> Proof<String> {
        let spelling = plain(path).ok_or("generic or qualified path is unsupported here")?;
        let mut parts = spelling.split("::");
        let root = parts.next().ok_or("empty path")?;
        let imports = self.imports(file)?;
        let suffix = parts.collect::<Vec<_>>().join("::");
        if let Some(import) = imports.get(root) {
            if let Some(standard) = import
                .split("::")
                .next()
                .filter(|root| matches!(*root, "std" | "core" | "alloc"))
            {
                need(
                    self.index.named_items(standard).is_empty() && !imports.contains_key(standard),
                    "imported standard root is shadowed",
                )?;
            }
            return Ok(if suffix.is_empty() {
                import.clone()
            } else {
                format!("{import}::{suffix}")
            });
        }
        if matches!(root, "std" | "core" | "alloc") {
            need(
                self.index.named_items(root).is_empty(),
                "standard root is shadowed",
            )?;
            return Ok(spelling);
        }
        if root == "crate" {
            return Ok(spelling);
        }
        if self.index.files[file].items.iter().any(|item| match item {
            Item::Struct(item) => item.ident == root,
            Item::Enum(item) => item.ident == root,
            Item::Trait(item) => item.ident == root,
            _ => false,
        }) {
            return Ok(format!(
                "{}::{root}{}",
                module(file)?,
                if suffix.is_empty() {
                    String::new()
                } else {
                    format!("::{suffix}")
                }
            ));
        }
        if matches!(
            root,
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
                | "Option"
                | "Vec"
                | "Box"
        ) {
            need(
                self.index.named_items(root).is_empty(),
                "standard type is shadowed",
            )?;
            let root = match root {
                "Option" => "std::option::Option",
                "Vec" => "std::vec::Vec",
                "Box" => "std::boxed::Box",
                _ => root,
            };
            return Ok(if suffix.is_empty() {
                root.into()
            } else {
                format!("{root}::{suffix}")
            });
        }
        Err(format!("unresolved source identity `{spelling}` in {file}"))
    }
    fn type_key(&self, file: &str, ty: &Type) -> Proof<String> {
        match ty {
            Type::Reference(value) => Ok(format!(
                "&{}{}{}",
                value
                    .lifetime
                    .as_ref()
                    .map(|value| format!("'{}", value.ident))
                    .unwrap_or_default(),
                if value.mutability.is_some() {
                    "mut "
                } else {
                    ""
                },
                self.type_key(file, &value.elem)?
            )),
            Type::Path(value) if value.qself.is_none() => {
                let mut path = value.path.clone();
                let args = path.segments.last_mut().ok_or("empty type path")?;
                let arguments = std::mem::replace(&mut args.arguments, syn::PathArguments::None);
                let key = self.reference(file, &path)?;
                // Type aliases do not establish identity in this opt-in proof.
                for (source, item) in self.index.items() {
                    if let Item::Type(item) = item {
                        need(
                            key != format!("{}::{}", module(source)?, item.ident),
                            "type aliases are unsupported in envelope flow",
                        )?;
                    }
                }
                match arguments {
                    syn::PathArguments::None => Ok(key),
                    syn::PathArguments::AngleBracketed(arguments) => {
                        let mut values = Vec::new();
                        for value in &arguments.args {
                            let syn::GenericArgument::Type(value) = value else {
                                return Err("unsupported type argument".into());
                            };
                            values.push(self.type_key(file, value)?);
                        }
                        Ok(format!("{key}<{}>", values.join(",")))
                    }
                    _ => Err("unsupported type form".into()),
                }
            }
            _ => Err("unsupported projection type form".into()),
        }
    }
    fn source_type(&self, name: &str) -> Proof<(String, &'a syn::ItemStruct)> {
        match self.index.named_items(name).as_slice() {
            [(file, Item::Struct(item))]
                if item.generics.params.is_empty() && item.generics.where_clause.is_none() =>
            {
                Ok((file.to_string(), item))
            }
            _ => Err(format!(
                "{name} must be one source-owned, non-generic struct"
            )),
        }
    }
    fn callable(&self, symbol: &str) -> Proof<Callable<'a>> {
        let (file, name) = symbol
            .split_once('#')
            .ok_or("callable must have exact path#symbol")?;
        // Retain the existing exact owner, attribute and generic signature guard.
        let signature = self
            .index
            .signature(symbol)
            .ok_or("callable signature is not uniquely source-owned")?;
        need(
            signature.generics.params.is_empty()
                && signature.generics.where_clause.is_none()
                && signature.unsafety.is_none()
                && signature.abi.is_none()
                && signature.variadic.is_none(),
            "generic, unsafe or foreign flow callable",
        )?;
        let mut found = Vec::new();
        for item in &self
            .index
            .files
            .get(file)
            .ok_or("callable file missing")?
            .items
        {
            match item {
                Item::Fn(item) if item.sig.ident == name => {
                    need(
                        item.attrs.iter().all(|attr| attr.path().is_ident("doc")),
                        "conditional or transformed flow callable",
                    )?;
                    found.push(Callable {
                        file: self.index.files.get_key_value(file).unwrap().0,
                        sig: &item.sig,
                        body: &item.block,
                        vis: &item.vis,
                    });
                }
                Item::Impl(item) if item.trait_.is_none() => {
                    if let Some((owner, method)) = name.split_once("::") {
                        if matches!(item.self_ty.as_ref(), Type::Path(path) if path.path.is_ident(owner))
                        {
                            for member in &item.items {
                                if let syn::ImplItem::Fn(member) = member {
                                    if member.sig.ident == method {
                                        found.push(Callable {
                                            file: self.index.files.get_key_value(file).unwrap().0,
                                            sig: &member.sig,
                                            body: &member.block,
                                            vis: &member.vis,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        need(found.len() == 1, "flow callable requires one concrete body")?;
        Ok(found.remove(0))
    }
    fn expect_type(&self, file: &str, ty: &Type, source: &str) -> Proof<()> {
        let (path, name) = source
            .split_once('#')
            .ok_or("invalid source type identity")?;
        need(
            self.type_key(file, ty)? == format!("{}::{name}", module(path)?),
            "wrong or aliased source type",
        )
    }
    fn permit_type(&mut self, ty: &Type) {
        if let Type::Path(path) = ty {
            self.allowed_types.insert(address(path));
        }
    }
    fn call(
        &mut self,
        file: &str,
        expr: &'a Expr,
        symbol: &str,
        awaited: bool,
    ) -> Proof<&'a syn::ExprCall> {
        let expr = if awaited {
            match expr {
                Expr::Await(value) => value.base.as_ref(),
                _ => return Err("exact awaited call required".into()),
            }
        } else {
            expr
        };
        let Expr::Call(call) = expr else {
            return Err("direct registered call required".into());
        };
        let Expr::Path(path) = call.func.as_ref() else {
            return Err("callable aliases and indirect calls are unsupported".into());
        };
        let (source, name) = symbol.split_once('#').ok_or("invalid registered symbol")?;
        need(
            path.qself.is_none()
                && self.reference(file, &path.path)? == format!("{}::{name}", module(source)?),
            "call does not resolve to its registered source owner",
        )?;
        self.allowed_paths.insert(address(path));
        Ok(call)
    }
    fn resource(&self, file: &str, ty: &Type) -> bool {
        if let Type::Reference(value) = ty {
            return self.resource(file, &value.elem);
        }
        let Type::Path(value) = ty else {
            return false;
        };
        let mut path = value.path.clone();
        let Some(last) = path.segments.last_mut() else {
            return false;
        };
        let arguments = std::mem::replace(&mut last.arguments, syn::PathArguments::None);
        let Ok(key) = self.reference(file, &path) else {
            return false;
        };
        match arguments {
            syn::PathArguments::None => self.index.items().any(|(source, item)| match item {
                Item::Struct(item) => {
                    !self.protected.contains(&item.ident.to_string())
                        && module(source)
                            .is_ok_and(|source| key == format!("{source}::{}", item.ident))
                }
                _ => false,
            }),
            syn::PathArguments::AngleBracketed(arguments)
                if matches!(key.as_str(), "std::sync::Arc" | "std::boxed::Box")
                    && arguments.args.len() == 1 =>
            {
                matches!(&arguments.args[0], syn::GenericArgument::Type(ty) if self.resource(file, ty))
            }
            _ => false,
        }
    }
}

pub(super) fn prove<'a>(
    index: &'a Index<'a>,
    flow: &EnvelopeFlow,
    executors: &[ExecutorBinding],
    types: &Types<'_>,
    contract: &FlowContract,
    asynchronous: bool,
) -> Proof<()> {
    let (context_file, context_name) = flow
        .context
        .split_once('#')
        .ok_or("context requires exact path#type")?;
    need(
        !context_name.contains("::"),
        "context must be a concrete module-level type",
    )?;
    let mut checker = Checker {
        index,
        protected: [
            context_name,
            types.effect,
            types.result,
            types.request_envelope,
            types.result_envelope,
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        callable_leaves: executors
            .iter()
            .map(|executor| executor.symbol.as_str())
            .chain([flow.split.as_str(), flow.observe.as_str()])
            .map(|symbol| {
                symbol
                    .rsplit("::")
                    .next()
                    .unwrap_or(symbol)
                    .rsplit('#')
                    .next()
                    .unwrap_or(symbol)
                    .to_string()
            })
            .collect(),
        allowed_paths: BTreeSet::new(),
        allowed_types: BTreeSet::new(),
        allowed_structs: BTreeSet::new(),
        allowed_methods: BTreeSet::new(),
        context_impl: 0,
    };
    let required = executors
        .iter()
        .map(|executor| executor.symbol.as_str())
        .chain(
            executors
                .iter()
                .filter_map(|executor| executor.status_symbol.as_deref()),
        )
        .chain([
            flow.context.as_str(),
            flow.split.as_str(),
            flow.observe.as_str(),
            flow.dispatcher.as_str(),
        ])
        .map(|symbol| {
            symbol
                .split_once('#')
                .map(|(file, _)| file)
                .ok_or("every flow symbol requires exact source path".into())
        })
        .collect::<Proof<BTreeSet<_>>>()?;
    let test_sources = checker.modules(&required)?;
    let (file, context) = checker.source_type(context_name)?;
    need(
        file == context_file
            && context.attrs.iter().all(|attr| attr.path().is_ident("doc"))
            && !matches!(context.vis, syn::Visibility::Public(_)),
        "context must be opaque, non-Clone/non-Copy and source-owned",
    )?;
    need(
        context
            .fields
            .iter()
            .all(|field| matches!(field.vis, syn::Visibility::Inherited)),
        "context fields must be private",
    )?;
    let context_fields = fields(&context.fields)?;
    let (request_file, request) = checker.source_type(types.request_envelope)?;
    let (result_file, result) = checker.source_type(types.result_envelope)?;
    let request_fields = fields(&request.fields)?;
    let result_fields = fields(&result.fields)?;
    let enum_source = |name: &str| -> Proof<String> {
        match index.named_items(name).as_slice() {
            [(file, Item::Enum(item))]
                if item.generics.params.is_empty()
                    && item.generics.where_clause.is_none()
                    && item.attrs.iter().all(inert_type_attribute) =>
            {
                Ok(format!("{file}#{name}"))
            }
            _ => Err("payload must be one non-generic closed source enum".into()),
        }
    };
    let effect_source = enum_source(types.effect)?;
    let status_source = enum_source(types.result)?;
    checker.modules(&BTreeSet::from([
        request_file.as_str(),
        result_file.as_str(),
        effect_source.split_once('#').unwrap().0,
        status_source.split_once('#').unwrap().0,
    ]))?;
    let payload_field =
        |file: &str, fields: &BTreeMap<String, &Type>, source: &str| -> Proof<String> {
            let matched = fields
                .iter()
                .filter(|(_, ty)| checker.expect_type(file, ty, source).is_ok())
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>();
            need(
                matched.len() == 1,
                "envelope must have one exact payload field",
            )?;
            Ok(matched[0].clone())
        };
    let effect_field = payload_field(&request_file, &request_fields, &effect_source)?;
    let status_field = payload_field(&result_file, &result_fields, &status_source)?;
    let mut declared_request = contract.request_fields.clone();
    declared_request.insert(effect_field.clone());
    need(
        request_fields.keys().cloned().collect::<BTreeSet<_>>() == declared_request,
        "request fields differ from canonical message fields",
    )?;
    need(
        result_fields.keys().cloned().collect::<BTreeSet<_>>() == contract.result_fields,
        "result fields differ from canonical message fields",
    )?;
    let identities = result_fields
        .keys()
        .filter(|name| **name != status_field)
        .cloned()
        .collect::<BTreeSet<_>>();
    need(
        !identities.is_empty()
            && context_fields.keys().cloned().collect::<BTreeSet<_>>() == identities,
        "context must retain every exact result identity field and nothing else",
    )?;
    for name in &identities {
        let input = request_fields
            .get(name)
            .ok_or("result identity is absent from request")?;
        need(
            checker.type_key(&request_file, input)?
                == checker.type_key(context_file, context_fields[name])?
                && checker.type_key(context_file, context_fields[name])?
                    == checker.type_key(&result_file, result_fields[name])?,
            "identity field type differs between request, context and result",
        )?;
    }
    let split = checker.callable(&flow.split)?;
    let observe = checker.callable(&flow.observe)?;
    let (_, split_name) = flow.split.split_once('#').unwrap();
    let (_, observe_name) = flow.observe.split_once('#').unwrap();
    need(
        flow.split
            .starts_with(&format!("{context_file}#{context_name}::"))
            && flow
                .observe
                .starts_with(&format!("{context_file}#{context_name}::"))
            && flow.split != flow.observe,
        "split and observe must be distinct inherent context methods",
    )?;
    let implementations = index.files[context_file].items.iter().filter_map(|item| match item {
        Item::Impl(item) if matches!(item.self_ty.as_ref(), Type::Path(path) if path.path.is_ident(context_name)) => Some(item), _ => None,
    }).collect::<Vec<_>>();
    need(
        implementations.len() == 1,
        "context requires one closed inherent impl",
    )?;
    let implementation = implementations[0];
    checker.context_impl = address(implementation);
    need(implementation.trait_.is_none() && implementation.attrs.is_empty() && implementation.items.len() == 2
        && implementation.items.iter().all(|item| matches!(item, syn::ImplItem::Fn(item) if item.sig.ident == split_name.split("::").last().unwrap() || item.sig.ident == observe_name.split("::").last().unwrap())), "extra context constructors, trait impls or members are unsupported")?;
    checker.permit_type(&implementation.self_ty);
    need(
        split.sig.asyncness.is_none() && split.sig.inputs.len() == 1,
        "split must consume one request synchronously",
    )?;
    let (request_name, request_ty) = argument(&split.sig.inputs[0])?;
    checker.expect_type(
        split.file,
        request_ty,
        &format!("{request_file}#{}", types.request_envelope),
    )?;
    let ReturnType::Type(_, split_output) = &split.sig.output else {
        return Err("split output missing".into());
    };
    let Type::Tuple(output) = split_output.as_ref() else {
        return Err("split must return (Self, Effect)".into());
    };
    need(
        output.elems.len() == 2
            && matches!(&output.elems[0], Type::Path(path) if path.qself.is_none() && path.path.is_ident("Self")),
        "split must return (Self, Effect)",
    )?;
    checker.expect_type(split.file, &output.elems[1], &effect_source)?;
    let Expr::Tuple(pair) = tail(split.body)? else {
        return Err("split requires a direct tuple construction".into());
    };
    need(
        pair.elems.len() == 2 && projection(&pair.elems[1], &request_name, &effect_field),
        "split must move the exact request payload",
    )?;
    let Expr::Struct(projected) = &pair.elems[0] else {
        return Err("split context construction missing".into());
    };
    need(
        projected.path.is_ident("Self"),
        "split must construct its exact Self context",
    )?;
    let projected_fields = literal_fields(projected)?;
    need(
        projected_fields.keys().cloned().collect::<BTreeSet<_>>() == identities
            && identities
                .iter()
                .all(|field| projection(projected_fields[field], &request_name, field)),
        "split identity fields must be unchanged direct request projections",
    )?;
    checker.allowed_structs.insert(address(projected));
    need(
        observe.sig.asyncness.is_none()
            && observe.sig.inputs.len() == 2
            && matches!(&observe.sig.inputs[0], FnArg::Receiver(receiver) if receiver.reference.is_none() && receiver.mutability.is_none() && receiver.colon_token.is_none()),
        "observe must consume Self and one status",
    )?;
    let (status_name, status_ty) = argument(&observe.sig.inputs[1])?;
    checker.expect_type(observe.file, status_ty, &status_source)?;
    checker.expect_type(
        observe.file,
        super::return_type(observe.sig).ok_or("observe output missing")?,
        &format!("{result_file}#{}", types.result_envelope),
    )?;
    let Expr::Struct(observed) = tail(observe.body)? else {
        return Err("observe requires direct result construction".into());
    };
    need(
        checker.reference(observe.file, &observed.path)?
            == format!("{}::{}", module(&result_file)?, types.result_envelope),
        "observe constructs a foreign result",
    )?;
    let observed_fields = literal_fields(observed)?;
    need(
        observed_fields.keys().cloned().collect::<BTreeSet<_>>() == contract.result_fields
            && identities
                .iter()
                .all(|field| projection(observed_fields[field], "self", field))
            && atom(observed_fields[&status_field], &status_name),
        "result must preserve exact identities and actual status",
    )?;
    checker.allowed_structs.insert(address(observed));
    let mut executor_parameters = BTreeMap::new();
    for executor in executors {
        let callable = checker.callable(&executor.symbol)?;
        need(
            !matches!(callable.vis, syn::Visibility::Public(_))
                && callable.sig.asyncness.is_some() == asynchronous,
            "projected executors must be crate-private and have the selected execution mode",
        )?;
        checker.expect_type(
            callable.file,
            super::return_type(callable.sig).ok_or("executor result missing")?,
            &format!("{result_file}#{}", types.result_envelope),
        )?;
        let (context_name, context_ty) = argument(
            callable
                .sig
                .inputs
                .iter()
                .nth(executor.request_parameter)
                .ok_or("context parameter index out of range")?,
        )?;
        checker.expect_type(callable.file, context_ty, &flow.context)?;
        checker.permit_type(context_ty);
        let status_symbol = executor
            .status_symbol
            .as_deref()
            .ok_or("each projected executor requires status_symbol")?;
        let status_callable = checker.callable(status_symbol)?;
        need(
            asynchronous || status_callable.sig.asyncness.is_none(),
            "synchronous envelope flow cannot await a status operation",
        )?;
        checker.expect_type(
            status_callable.file,
            super::return_type(status_callable.sig).ok_or("status output missing")?,
            &status_source,
        )?;
        let (status_expression, result_expression, status_binding) = match callable.body.stmts.as_slice() {
            [Stmt::Local(local), Stmt::Expr(result, None)] if local.attrs.is_empty() => {
                let binding = identifier(&local.pat).ok_or("status binding must be immutable")?;
                let init = local.init.as_ref().ok_or("status initialization missing")?;
                need(init.diverge.is_none(), "status let-else is unsupported")?;
                (init.expr.as_ref(), result, Some(binding))
            }
            [Stmt::Expr(result, None)] => (result, result, None),
            _ => return Err("executor must return its one status call through observe; extra statements or control flow are unsupported".into()),
        };
        let Expr::MethodCall(observe_call) = result_expression else {
            return Err("executor must end in context.observe(actual_status)".into());
        };
        need(
            atom(&observe_call.receiver, &context_name)
                && observe_call.method == observe_name.split("::").last().unwrap()
                && observe_call.turbofish.is_none()
                && observe_call.args.len() == 1,
            "executor must consume the same context in registered observe",
        )?;
        let status_expression = if let Some(binding) = status_binding {
            need(
                binding != context_name && atom(&observe_call.args[0], &binding),
                "executor substituted or shadowed its actual status",
            )?;
            status_expression
        } else {
            &observe_call.args[0]
        };
        let operation = checker.call(
            callable.file,
            status_expression,
            status_symbol,
            status_callable.sig.asyncness.is_some(),
        )?;
        checker.allowed_methods.insert(address(observe_call));
        let params = callable
            .sig
            .inputs
            .iter()
            .map(argument)
            .collect::<Proof<Vec<_>>>()?;
        need(
            params
                .iter()
                .map(|(name, _)| name)
                .collect::<BTreeSet<_>>()
                .len()
                == params.len(),
            "duplicate executor parameter",
        )?;
        let mut consumed = BTreeSet::new();
        for arg in &operation.args {
            let arg = match arg {
                Expr::Reference(value) if value.mutability.is_none() => value.expr.as_ref(),
                _ => arg,
            };
            let Expr::Path(path) = arg else {
                return Err("status call arguments must be exact executor parameters".into());
            };
            need(
                path.qself.is_none(),
                "qualified-self status arguments are unsupported",
            )?;
            let name = plain(&path.path).ok_or("indirect status argument")?;
            need(
                name != context_name
                    && params.iter().any(|(parameter, _)| parameter == &name)
                    && consumed.insert(name),
                "status call must consume each operation parameter once, never context",
            )?;
        }
        need(
            consumed.len() + 1 == params.len(),
            "status call dropped an operation parameter",
        )?;
        executor_parameters.insert(executor.symbol.clone(), params);
    }
    let dispatcher = checker.callable(&flow.dispatcher)?;
    let (issued_name, issued_type) = argument(
        dispatcher
            .sig
            .inputs
            .iter()
            .nth(flow.request_parameter)
            .ok_or("dispatcher request index out of range")?,
    )?;
    checker.expect_type(
        dispatcher.file,
        issued_type,
        &format!("{request_file}#{}", types.request_envelope),
    )?;
    let mut resources = BTreeSet::new();
    for (position, param) in dispatcher.sig.inputs.iter().enumerate() {
        if position == flow.request_parameter {
            continue;
        }
        let (name, ty) = argument(param)?;
        need(
            checker.resource(dispatcher.file, ty) && resources.insert(name),
            "dispatcher extra parameters must be exact source-owned resources",
        )?;
    }
    let output = super::return_type(dispatcher.sig).ok_or("dispatcher output missing")?;
    let body = if asynchronous && dispatcher.sig.asyncness.is_none() {
        checker.expect_type(
            dispatcher.file,
            index
                .future_output(output, 0)
                .ok_or("dispatcher must return exact envelope future")?,
            &format!("{result_file}#{}", types.result_envelope),
        )?;
        let Expr::Call(pin) = tail(dispatcher.body)? else {
            return Err("dispatcher requires Box::pin(async move { ... })".into());
        };
        let Expr::Path(path) = pin.func.as_ref() else {
            return Err("dispatcher pin must be direct".into());
        };
        need(
            checker.reference(dispatcher.file, &path.path)? == "std::boxed::Box::pin"
                && pin.args.len() == 1,
            "dispatcher requires standard Box::pin",
        )?;
        let Expr::Async(value) = &pin.args[0] else {
            return Err("dispatcher requires async move".into());
        };
        need(
            value.capture.is_some(),
            "dispatcher future must own its request",
        )?;
        &value.block
    } else {
        need(
            dispatcher.sig.asyncness.is_some() == asynchronous,
            "dispatcher mode mismatch",
        )?;
        checker.expect_type(
            dispatcher.file,
            output,
            &format!("{result_file}#{}", types.result_envelope),
        )?;
        dispatcher.body
    };
    let [Stmt::Local(split_binding), Stmt::Expr(Expr::Match(dispatch), None)] =
        body.stmts.as_slice()
    else {
        return Err(
            "dispatcher must split once and immediately exhaustively match its effect".into(),
        );
    };
    need(split_binding.attrs.is_empty(), "transformed split binding")?;
    let Pat::Tuple(names) = &split_binding.pat else {
        return Err("dispatcher split requires two immutable bindings".into());
    };
    need(names.elems.len() == 2, "split binding arity")?;
    let context_binding = identifier(&names.elems[0]).ok_or("context binding must be immutable")?;
    let effect_binding = identifier(&names.elems[1]).ok_or("effect binding must be immutable")?;
    need(
        context_binding != effect_binding
            && context_binding != issued_name
            && effect_binding != issued_name
            && !resources.contains(&context_binding)
            && !resources.contains(&effect_binding),
        "dispatcher shadows its issued request or resources",
    )?;
    let init = split_binding.init.as_ref().ok_or("split call missing")?;
    need(init.diverge.is_none(), "split let-else unsupported")?;
    let split_call = checker.call(dispatcher.file, &init.expr, &flow.split, false)?;
    need(
        split_call.args.len() == 1
            && atom(&split_call.args[0], &issued_name)
            && atom(&dispatch.expr, &effect_binding),
        "dispatcher must split its original request and match that exact effect",
    )?;
    let effect_items = index.named_items(types.effect);
    let effect = match effect_items.as_slice() {
        [(_, Item::Enum(effect))] => effect,
        _ => return Err("closed effect enum missing".into()),
    };
    let variants = effect
        .variants
        .iter()
        .map(|variant| (variant.ident.to_string(), variant))
        .collect::<BTreeMap<_, _>>();
    need(
        variants.len() == effect.variants.len()
            && contract.protocols.keys().cloned().collect::<BTreeSet<_>>()
                == executors
                    .iter()
                    .map(|executor| executor.symbol.clone())
                    .collect()
            && contract
                .protocols
                .values()
                .cloned()
                .collect::<BTreeSet<_>>()
                == variants.keys().cloned().collect()
            && contract.protocols.len() == variants.len(),
        "canonical protocols must bijectively cover every source effect variant and executor",
    )?;
    let effect_identity = format!(
        "{}::{}",
        module(effect_source.split_once('#').unwrap().0)?,
        types.effect
    );
    let mut covered = BTreeSet::new();
    for arm in &dispatch.arms {
        need(
            arm.attrs.is_empty() && arm.guard.is_none(),
            "conditional or guarded dispatch arms are unsupported",
        )?;
        let (path, bindings) = match &arm.pat {
            Pat::Path(value) if value.qself.is_none() => (&value.path, Vec::new()),
            Pat::TupleStruct(value) if value.qself.is_none() => (
                &value.path,
                value
                    .elems
                    .iter()
                    .enumerate()
                    .map(|(position, pat)| {
                        identifier(pat)
                            .map(|name| (position.to_string(), name))
                            .ok_or("tuple payload must bind every field exactly once".into())
                    })
                    .collect::<Proof<Vec<_>>>()?,
            ),
            Pat::Struct(value) if value.qself.is_none() && value.rest.is_none() => (
                &value.path,
                value
                    .fields
                    .iter()
                    .map(|field| match &field.member {
                        syn::Member::Named(member) if field.attrs.is_empty() => {
                            identifier(&field.pat)
                                .map(|name| (member.to_string(), name))
                                .ok_or("payload fields must be immutable identifiers".into())
                        }
                        _ => Err("unsupported payload field pattern".into()),
                    })
                    .collect::<Proof<Vec<_>>>()?,
            ),
            _ => return Err(
                "dispatch requires exact variants with no wildcard, alias, rest or nested pattern"
                    .into(),
            ),
        };
        let resolved = checker.reference(dispatcher.file, path)?;
        let variant_name = resolved
            .strip_prefix(&format!("{effect_identity}::"))
            .ok_or("foreign or aliased effect variant")?;
        let variant = variants
            .get(variant_name)
            .ok_or("undeclared effect variant")?;
        need(
            matches!(
                (&variant.fields, &arm.pat),
                (syn::Fields::Unit, Pat::Path(_))
                    | (syn::Fields::Named(_), Pat::Struct(_))
                    | (syn::Fields::Unnamed(_), Pat::TupleStruct(_))
            ),
            "pattern shape differs from the exact variant payload",
        )?;
        need(
            variant.attrs.iter().all(|attr| attr.path().is_ident("doc"))
                && covered.insert(variant_name.to_string()),
            "conditional or duplicate dispatch variant",
        )?;
        let payload = match &variant.fields {
            syn::Fields::Unit => BTreeMap::new(),
            syn::Fields::Named(_) => fields(&variant.fields)?,
            syn::Fields::Unnamed(fields) => {
                need(
                    fields.unnamed.iter().all(|field| field.attrs.is_empty()),
                    "conditional tuple payload",
                )?;
                fields
                    .unnamed
                    .iter()
                    .enumerate()
                    .map(|(position, field)| (position.to_string(), &field.ty))
                    .collect()
            }
        };
        need(
            bindings
                .iter()
                .map(|(field, _)| field)
                .collect::<BTreeSet<_>>()
                .len()
                == payload.len()
                && bindings.len() == payload.len()
                && bindings
                    .iter()
                    .all(|(field, _)| payload.contains_key(field)),
            "dispatch dropped or duplicated a payload field",
        )?;
        let mut payload_names = BTreeMap::new();
        for (field, name) in &bindings {
            need(
                name != &context_binding
                    && name != &effect_binding
                    && name != &issued_name
                    && !resources.contains(name)
                    && payload_names.insert(name.clone(), payload[field]).is_none(),
                "payload shadows context or another origin",
            )?;
        }
        let executor = executors
            .iter()
            .find(|executor| {
                contract
                    .protocols
                    .get(&executor.symbol)
                    .is_some_and(|name| name == variant_name)
            })
            .ok_or("variant executor missing")?;
        let expression = match arm.body.as_ref() {
            Expr::Block(block) if block.label.is_none() => tail(&block.block)?,
            value => value,
        };
        let call = checker.call(dispatcher.file, expression, &executor.symbol, asynchronous)?;
        let params = &executor_parameters[&executor.symbol];
        need(
            call.args.len() == params.len(),
            "executor argument arity mismatch",
        )?;
        let mut used = BTreeSet::new();
        let executor_file = executor.symbol.split_once('#').unwrap().0;
        for (position, arg) in call.args.iter().enumerate() {
            if position == executor.request_parameter {
                need(
                    atom(arg, &context_binding),
                    "executor receives a foreign or forged context",
                )?;
                continue;
            }
            let Expr::Path(path) = arg else {
                return Err(
                    "executor arguments must be moved resource or exact matched payload bindings"
                        .into(),
                );
            };
            need(
                path.qself.is_none(),
                "qualified-self executor arguments are unsupported",
            )?;
            let name = plain(&path.path).ok_or("executor argument alias")?;
            if let Some(ty) = payload_names.get(&name) {
                need(
                    used.insert(name)
                        && checker.type_key(effect_source.split_once('#').unwrap().0, ty)?
                            == checker.type_key(executor_file, params[position].1)?,
                    "payload was duplicated, substituted or mistyped",
                )?;
            } else {
                need(
                    resources.contains(&name)
                        && checker.resource(executor_file, params[position].1),
                    "executor receives an unrelated payload",
                )?;
            }
        }
        need(
            used.len() == payload_names.len(),
            "executor dropped matched payload",
        )?;
    }
    need(
        covered.len() == variants.len(),
        "dispatch is not exhaustive",
    )?;
    let mut closure = Closure {
        checker: &checker,
        context_name,
        file: String::new(),
        declarations: BTreeMap::new(),
        reason: None,
    };
    for (name, file) in index.files {
        if test_sources.contains(name) {
            continue;
        }
        closure.file = name.clone();
        closure.visit_file(file);
    }
    if let Some(reason) = closure.reason {
        return Err(reason);
    }
    need(
        checker
            .protected
            .iter()
            .all(|name| closure.declarations.get(name) == Some(&1)),
        "nested, duplicate or shadowed proof types",
    )?;
    Ok(())
}

fn inert_type_attribute(attr: &syn::Attribute) -> bool {
    if attr.path().is_ident("doc") {
        return true;
    }
    if attr.path().is_ident("derive") {
        return attr
            .parse_args_with(
                syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated,
            )
            .is_ok_and(|paths| {
                paths.iter().all(|path| {
                    plain(path).is_some_and(|path| {
                        matches!(
                            path.as_str(),
                            "Clone"
                                | "Copy"
                                | "Debug"
                                | "Eq"
                                | "PartialEq"
                                | "Ord"
                                | "PartialOrd"
                                | "Hash"
                                | "Default"
                        )
                    })
                })
            });
    }
    false
}
struct Closure<'a, 'b> {
    checker: &'b Checker<'a>,
    context_name: &'b str,
    file: String,
    declarations: BTreeMap<String, usize>,
    reason: Option<String>,
}
impl Closure<'_, '_> {
    fn refuse(&mut self, reason: &str) {
        if self.reason.is_none() {
            self.reason = Some(if reason.starts_with(&format!("{}: ", self.file)) {
                reason.into()
            } else {
                format!("{}: {reason}", self.file)
            });
        }
    }
    fn sensitive(&self, path: &syn::Path) -> bool {
        path.segments
            .iter()
            .any(|part| self.checker.protected.contains(&part.ident.to_string()))
    }
    fn declaration(&mut self, name: &syn::Ident) {
        if self.checker.protected.contains(&name.to_string()) {
            *self.declarations.entry(name.to_string()).or_default() += 1;
        }
    }
}
impl<'ast> Visit<'ast> for Closure<'_, '_> {
    fn visit_item_struct(&mut self, node: &'ast syn::ItemStruct) {
        self.declaration(&node.ident);
        visit::visit_item_struct(self, node);
    }
    fn visit_item_enum(&mut self, node: &'ast syn::ItemEnum) {
        self.declaration(&node.ident);
        visit::visit_item_enum(self, node);
    }
    fn visit_item_type(&mut self, node: &'ast syn::ItemType) {
        self.declaration(&node.ident);
        visit::visit_item_type(self, node);
    }
    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        self.declaration(&node.ident);
        visit::visit_item_trait(self, node);
    }
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        self.declaration(&node.ident);
        if test_only_module(node) {
            // Ignore only the inert test placement attribute. Keep any inspected
            // inline contents in the same strict provenance closure as before.
            for attr in &node.attrs {
                if !attr.path().is_ident("path") {
                    self.visit_attribute(attr);
                }
            }
            self.visit_visibility(&node.vis);
            self.visit_ident(&node.ident);
            if let Some((_, items)) = &node.content {
                for item in items {
                    self.visit_item(item);
                }
            }
            return;
        }
        visit::visit_item_mod(self, node);
    }
    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        // Validate the root import scope before visiting its attributes. This
        // preserves the actual import error even for a custom attribute.
        if self.checker.index.files[&self.file]
            .items
            .iter()
            .any(|item| matches!(item, Item::Use(item) if address(item) == address(node)))
        {
            if let Err(reason) = self.checker.imports(&self.file) {
                self.refuse(&reason);
            }
        }
        visit::visit_item_use(self, node);
    }
    fn visit_type_param(&mut self, node: &'ast syn::TypeParam) {
        if self.checker.protected.contains(&node.ident.to_string()) {
            self.refuse("generic parameter shadows a flow type");
        }
        visit::visit_type_param(self, node);
    }
    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        if node
            .path
            .segments
            .iter()
            .any(|part| part.ident == self.context_name)
            && !self.checker.allowed_types.contains(&address(node))
        {
            self.refuse("context escapes through an alias, field or unregistered signature");
        }
        visit::visit_type_path(self, node);
    }
    fn visit_expr_struct(&mut self, node: &'ast syn::ExprStruct) {
        if self.sensitive(&node.path)
            && node
                .path
                .segments
                .iter()
                .any(|part| part.ident == self.context_name)
            && !self.checker.allowed_structs.contains(&address(node))
        {
            self.refuse("extra context constructor");
        }
        visit::visit_expr_struct(self, node);
    }
    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if matches!(node.self_ty.as_ref(), Type::Path(path) if path.path.segments.iter().any(|part| part.ident == self.context_name))
            && address(node) != self.checker.context_impl
        {
            self.refuse("extra context impl or Clone/Copy implementation");
        }
        visit::visit_item_impl(self, node);
    }
    fn visit_expr_path(&mut self, node: &'ast syn::ExprPath) {
        let sensitive = node
            .path
            .segments
            .iter()
            .any(|part| self.checker.protected.contains(&part.ident.to_string()));
        if sensitive && !self.checker.allowed_paths.contains(&address(node)) {
            // Result enum expressions belong to the status operations; the
            // context itself and registered callables may never escape.
            if node
                .path
                .segments
                .iter()
                .any(|part| part.ident == self.context_name)
            {
                self.refuse("context constructor or callable escapes the registered flow");
            }
        }
        let leaf = node
            .path
            .segments
            .last()
            .map(|part| part.ident.to_string())
            .unwrap_or_default();
        if self.checker.callable_leaves.contains(&leaf)
            && !self.checker.allowed_paths.contains(&address(node))
        {
            self.refuse("registered callable has an unproven call site or function alias");
        }
        visit::visit_expr_path(self, node);
    }
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if self
            .checker
            .callable_leaves
            .contains(&node.method.to_string())
            && !self.checker.allowed_methods.contains(&address(node))
        {
            self.refuse("registered context/executor method has an unproven call site");
        }
        visit::visit_expr_method_call(self, node);
    }
    fn visit_use_rename(&mut self, node: &'ast syn::UseRename) {
        if self.checker.protected.contains(&node.ident.to_string())
            || self.checker.protected.contains(&node.rename.to_string())
            || self
                .checker
                .callable_leaves
                .contains(&node.ident.to_string())
        {
            self.refuse("flow type or callable import alias");
        }
        visit::visit_use_rename(self, node);
    }
    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        if !inert_type_attribute(node)
            && !node.path().is_ident("cfg")
            && !node.path().is_ident("test")
        {
            self.refuse("unsupported source-transforming attribute in flow source closure");
        }
        if node.path().is_ident("derive") {
            match self.checker.imports(&self.file) {
                Err(reason) => self.refuse(&reason),
                Ok(imports) => {
                    for name in [
                        "Clone",
                        "Copy",
                        "Debug",
                        "Eq",
                        "PartialEq",
                        "Ord",
                        "PartialOrd",
                        "Hash",
                        "Default",
                    ] {
                        if imports.get(name).is_some_and(|target| {
                            !target.starts_with("std::") && !target.starts_with("core::")
                        }) {
                            self.refuse(&format!(
                                "standard derive `{name}` is shadowed by file-level import `{}`",
                                imports[name]
                            ));
                        }
                    }
                }
            }
        }
        visit::visit_attribute(self, node);
    }
    fn visit_item_macro(&mut self, _node: &'ast syn::ItemMacro) {
        self.refuse("source-generating macros prevent constructor closure");
    }
    fn visit_item_extern_crate(&mut self, _node: &'ast syn::ItemExternCrate) {
        self.refuse("extern-crate aliases prevent source identity closure");
    }
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        self.inspect_macro(node);
    }
}
impl Closure<'_, '_> {
    fn macro_refuse(&mut self, node: &syn::Macro, cause: &str) {
        let location = node.path.span().start();
        let path = node
            .path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        self.refuse(&format!(
            "macro `{path}!` at {}:{} token path `{path}`: {cause}",
            location.line,
            location.column + 1
        ));
    }
    fn inspect_macro(&mut self, node: &syn::Macro) {
        let spelling = plain(&node.path).unwrap_or_default();
        let leaf = spelling.rsplit("::").next().unwrap_or_default();
        if self.checker.index.named_items("std").len()
            + self.checker.index.named_items("core").len()
            > 0
        {
            self.macro_refuse(node, "macro standard root is shadowed");
        }
        match self.checker.imports(&self.file) {
            Ok(imports) if !imports.contains_key("std") && !imports.contains_key("core") => {
                if !spelling.contains("::")
                    && imports.get(leaf).is_some_and(|target| {
                        !target.starts_with("std::") && !target.starts_with("core::")
                    })
                {
                    self.macro_refuse(node, "macro identity is open or shadowed");
                }
            }
            Err(reason) => self.macro_refuse(node, &reason),
            _ => self.macro_refuse(node, "macro identity is open or shadowed"),
        }
        if !matches!(
            leaf,
            "matches"
                | "ready"
                | "pin"
                | "assert"
                | "assert_eq"
                | "assert_ne"
                | "debug_assert"
                | "debug_assert_eq"
                | "debug_assert_ne"
                | "vec"
                | "format"
                | "format_args"
                | "println"
                | "eprintln"
                | "print"
                | "eprint"
                | "panic"
                | "todo"
                | "unreachable"
                | "write"
                | "writeln"
                | "cfg"
                | "stringify"
                | "concat"
                | "include_str"
                | "include_bytes"
                | "env"
                | "option_env"
                | "line"
                | "file"
                | "column"
        ) || (spelling.contains("::")
            && !spelling.starts_with("std::")
            && !spelling.starts_with("core::"))
        {
            self.macro_refuse(node, "unsupported macro in flow source closure");
        }
        self.inspect_macro_tokens(node, node.tokens.clone(), "input");
        let format_index = match leaf {
            "format" | "format_args" | "print" | "println" | "eprint" | "eprintln" | "panic"
            | "todo" | "unreachable" => Some(0),
            "write" | "writeln" | "assert" | "debug_assert" => Some(1),
            "assert_eq" | "assert_ne" | "debug_assert_eq" | "debug_assert_ne" => Some(2),
            _ => None,
        };
        if let Some(index) = format_index {
            match syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                .parse2(node.tokens.clone())
            {
                Ok(arguments) => {
                    if let Some(expr) = arguments.iter().nth(index) {
                        if let Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(literal),
                            ..
                        }) = expr
                        {
                            match format_capture_names(&literal.value()) {
                                Ok(names) => {
                                    for (name, offset) in names {
                                        if self.protected_macro_identifier(&name) {
                                            self.macro_refuse(node, &format!("format argument token {index}, decoded field byte {offset}: capture `{name}` hides a flow type or registered callable"));
                                        }
                                    }
                                }
                                Err(reason) => self.macro_refuse(
                                    node,
                                    &format!("format argument token {index}: {reason}"),
                                ),
                            }
                        } else {
                            self.macro_refuse(node, &format!("format argument token {index}: require a literal format string; constructed formats are unsupported"));
                        }
                    }
                }
                Err(_) => self.macro_refuse(node, "unsupported formatting argument tokens"),
            }
        }
    }
    fn protected_macro_identifier(&self, name: &str) -> bool {
        let name = name.strip_prefix("r#").unwrap_or(name);
        self.checker.protected.contains(name) || self.checker.callable_leaves.contains(name)
    }
    fn inspect_macro_tokens(&mut self, owner: &syn::Macro, stream: TokenStream, position: &str) {
        let tokens = stream.into_iter().collect::<Vec<_>>();
        for (index, token) in tokens.iter().enumerate() {
            match token {
                TokenTree::Ident(ident) if self.protected_macro_identifier(&ident.to_string()) => {
                    let location = ident.span().start();
                    self.macro_refuse(owner, &format!("{position}[{index}] identifier `{ident}` at {}:{} hides a flow type or registered callable", location.line, location.column + 1));
                }
                TokenTree::Group(group) => {
                    let bang = |token: &TokenTree| matches!(token, TokenTree::Punct(punct) if punct.as_char() == '!');
                    let colon = |token: &TokenTree| matches!(token, TokenTree::Punct(punct) if punct.as_char() == ':');
                    if index >= 2
                        && bang(&tokens[index - 1])
                        && matches!(tokens[index - 2], TokenTree::Ident(_))
                    {
                        let mut start = index - 2;
                        while start >= 3
                            && colon(&tokens[start - 1])
                            && colon(&tokens[start - 2])
                            && matches!(tokens[start - 3], TokenTree::Ident(_))
                        {
                            start -= 3;
                        }
                        if start >= 2 && colon(&tokens[start - 1]) && colon(&tokens[start - 2]) {
                            start -= 2;
                        }
                        let nested = tokens[start..=index].iter().cloned().collect();
                        match syn::parse2::<syn::Macro>(nested) {
                            Ok(nested) => self.inspect_macro(&nested),
                            Err(_) => self.macro_refuse(
                                owner,
                                &format!("{position}[{index}]: unsupported nested macro tokens"),
                            ),
                        }
                    } else {
                        self.inspect_macro_tokens(
                            owner,
                            group.stream(),
                            &format!("{position}[{index}]"),
                        );
                    }
                }
                // Literals are data. Only an actual formatter's selected
                // literal is separately parsed for implicit identifier captures.
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const TYPES: &str = r#"
pub(crate) struct Payload(Vec<u8>);
pub(crate) enum Effect { Read { key: Payload }, Stop }
pub(crate) enum Status { Read(Payload), Stopped }
pub(crate) struct Request {
    pub effect_id: u64, pub requester: &'static str, pub correlation_id: u64,
    pub causation_id: Option<u64>, pub idempotency_key: Option<u64>, pub effect: Effect,
}
pub(crate) struct ResultEnvelope {
    pub effect_id: u64, pub requester: &'static str, pub correlation_id: u64,
    pub causation_id: Option<u64>, pub status: Status,
}
pub(crate) struct Context {
    effect_id: u64, requester: &'static str, correlation_id: u64, causation_id: Option<u64>,
}
impl Context {
    pub(crate) fn split(request: Request) -> (Self, Effect) {
        (Self { effect_id: request.effect_id, requester: request.requester,
            correlation_id: request.correlation_id, causation_id: request.causation_id }, request.effect)
    }
    pub(crate) fn observed(self, status: Status) -> ResultEnvelope {
        ResultEnvelope { effect_id: self.effect_id, requester: self.requester,
            correlation_id: self.correlation_id, causation_id: self.causation_id, status }
    }
}
pub(crate) struct State;
pub(crate) enum Input { Observed(ResultEnvelope) }
pub(crate) struct Record;
"#;
    const RUNTIME: &str = r#"
use crate::types::{Context, Effect, Input, Record, Request, ResultEnvelope, State};
use crate::io::Ports;
use std::{future::Future, pin::Pin, sync::Arc, task::Poll};
type Pending = Pin<Box<dyn Future<Output = ResultEnvelope> + Send>>;
struct Runtime { state: State, records: Vec<Record>, pending: Vec<Pending> }
trait Inputs { fn poll(&self) -> Poll<Option<Input>>; }
async fn drive(runtime: &mut Runtime) {}
fn dispatch(ports: Arc<Ports>, request: Request) -> Pending {
    Box::pin(async move {
        let (context, effect) = Context::split(request);
        match effect {
            Effect::Read { key } => crate::read::execute(ports, context, key).await,
            Effect::Stop => crate::stop::execute(ports, context).await,
        }
    })
}
"#;
    const READ: &str = r#"
use crate::types::{Context, Payload, ResultEnvelope};
use crate::io::Ports;
use std::sync::Arc;
pub(crate) async fn execute(ports: Arc<Ports>, context: Context, key: Payload) -> ResultEnvelope {
    let status = crate::io::Ports::read(&ports, key).await;
    context.observed(status)
}
"#;
    const STOP: &str = r#"
use crate::types::{Context, ResultEnvelope};
use crate::io::Ports;
use std::sync::Arc;
pub(crate) async fn execute(ports: Arc<Ports>, context: Context) -> ResultEnvelope {
    context.observed(crate::io::Ports::stop(&ports).await)
}
"#;
    const IO: &str = r#"
use crate::types::{Payload, Status};
use std::sync::Arc;
pub(crate) struct Ports;
impl Ports {
    pub(crate) async fn read(ports: &Arc<Ports>, key: Payload) -> Status { Status::Read(key) }
    pub(crate) async fn stop(ports: &Arc<Ports>) -> Status { Status::Stopped }
}
"#;
    const BINDING: &str = r#"
kind: persistent-async
runtime: src/runtime.rs#Runtime
state_field: state
records_field: records
pending_field: pending
input_poll: src/runtime.rs#Inputs::poll
executors:
- {symbol: 'src/read.rs#execute', request_parameter: 1, status_symbol: 'src/io.rs#Ports::read'}
- {symbol: 'src/stop.rs#execute', request_parameter: 1, status_symbol: 'src/io.rs#Ports::stop'}
envelope_flow:
  context: src/types.rs#Context
  split: src/types.rs#Context::split
  observe: src/types.rs#Context::observed
  dispatcher: src/runtime.rs#dispatch
  request_parameter: 1
"#;
    fn sources() -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                "src/lib.rs".into(),
                "mod types; mod runtime; mod read; mod stop; mod io;".into(),
            ),
            ("src/types.rs".into(), TYPES.into()),
            ("src/runtime.rs".into(), RUNTIME.into()),
            ("src/read.rs".into(), READ.into()),
            ("src/stop.rs".into(), STOP.into()),
            ("src/io.rs".into(), IO.into()),
        ])
    }
    fn contract() -> FlowContract {
        FlowContract {
            protocols: BTreeMap::from([
                ("src/read.rs#execute".into(), "Read".into()),
                ("src/stop.rs#execute".into(), "Stop".into()),
            ]),
            request_fields: [
                "effect_id",
                "requester",
                "correlation_id",
                "causation_id",
                "idempotency_key",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            result_fields: [
                "effect_id",
                "requester",
                "correlation_id",
                "causation_id",
                "status",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
        }
    }
    fn types() -> Types<'static> {
        Types {
            state: "State",
            input: "Input",
            record: "Record",
            effect: "Effect",
            result: "Status",
            request_envelope: "Request",
            result_envelope: "ResultEnvelope",
        }
    }
    fn check(
        sources: &BTreeMap<String, String>,
        yaml: &str,
        contract: &FlowContract,
    ) -> Vec<String> {
        let files = sources
            .iter()
            .map(|(file, source)| (file.clone(), syn::parse_file(source).unwrap()))
            .collect();
        let binding = serde_yaml::from_str(yaml).unwrap();
        Index { files: &files }.validate_with_flow(
            &binding,
            "src/runtime.rs#drive",
            &contract.protocols.keys().cloned().collect::<Vec<_>>(),
            &types(),
            Some(contract),
        )
    }
    #[test]
    fn test_only_module_exception_has_a_closed_attribute_grammar() {
        for declaration in [
            "#[cfg(test)] mod arbitrary_name;",
            "#[doc(hidden)] #[path = \"support.rs\"] #[cfg(test)] mod tests;",
            "#[cfg(test)] mod tests { #[test] fn smoke() {} }",
        ] {
            assert!(test_only_module(&syn::parse_str(declaration).unwrap()));
        }
        for declaration in [
            "mod tests;",
            "#[cfg(test)] #[cfg(test)] mod tests;",
            "#[cfg(test)] #[path = \"\"] mod tests;",
            "#[cfg(test)] #[path = 1] mod tests;",
            "#[cfg(test)] #[path = \"a.rs\"] #[path = \"b.rs\"] mod tests;",
            "#[other::cfg(test)] mod tests;",
            "#[cfg(test)] #[other::path = \"a.rs\"] mod tests;",
        ] {
            assert!(!test_only_module(&syn::parse_str(declaration).unwrap()));
        }
    }
    #[test]
    fn test_only_module_placement_preserves_production_flow_and_test_closure() {
        let mut moved = sources();
        moved.get_mut("src/io.rs").unwrap().push_str(
            "\n#[cfg(test)] #[path = \"../tests/support/io_support_unit.rs\"] mod tests;",
        );
        moved.insert("tests/support/io_support_unit.rs".into(),
            "use super::*; #[test] fn private_parent_access() { let _ = Ports; }".into());
        let errors = check(&moved, BINDING, &contract());
        assert!(errors.is_empty(), "{errors:?}");

        let mut ordinary = sources();
        ordinary.get_mut("src/io.rs").unwrap().push_str("\n#[cfg(test)] mod tests;");
        ordinary.insert("src/io/tests.rs".into(), "use super::*; #[test] fn smoke() {}".into());
        assert!(check(&ordinary, BINDING, &contract()).is_empty());
        let mut inline = sources();
        inline.get_mut("src/io.rs").unwrap().push_str(
            "\n#[cfg(test)] mod tests { use super::*; #[test] fn smoke() {} }",
        );
        assert!(check(&inline, BINDING, &contract()).is_empty());

        // A test reference cannot exempt a file that production also reaches.
        let mut dual = sources();
        dual.get_mut("src/lib.rs").unwrap().push_str(
            "\n#[cfg(test)] #[path = \"io.rs\"] mod tests;",
        );
        dual.get_mut("src/io.rs").unwrap().push_str(
            "\nfn bypass() { crate::types::Context::split(request); }",
        );
        assert!(!check(&dual, BINDING, &contract()).is_empty());

        for attrs in [
            "#[path = \"../tests/support/io_support_unit.rs\"]",
            "#[cfg(feature = \"tests\")] #[path = \"../tests/support/io_support_unit.rs\"]",
            "#[cfg(any(test, feature = \"tests\"))] #[path = \"../tests/support/io_support_unit.rs\"]",
            "#[cfg_attr(test, path = \"../tests/support/io_support_unit.rs\")]",
            "#[cfg(test)] #[custom] #[path = \"../tests/support/io_support_unit.rs\"]",
            "#[cfg(test)] #[path = \"/tmp/tests.rs\"]",
            "#[cfg(test)] #[path = \"../../tests.rs\"]",
        ] {
            let mut invalid = moved.clone();
            invalid.insert("src/io.rs".into(), format!("{IO}\n{attrs} mod tests;"));
            assert!(!check(&invalid, BINDING, &contract()).is_empty(), "{attrs}");
        }
        let mut hidden_owner = moved.clone();
        hidden_owner.insert("src/lib.rs".into(),
            "mod types; mod runtime; #[cfg(test)] mod read; mod stop; mod io;".into());
        assert!(check(&hidden_owner, BINDING, &contract()).iter()
            .any(|reason| reason.contains("registered source is not wired")));

        // An unrelated inspected file is not test-only merely by its path.
        moved.insert("tests/unregistered.rs".into(),
            "fn bypass() { crate::types::Context::split(request); }".into());
        assert!(!check(&moved, BINDING, &contract()).is_empty());
    }
    #[test]
    fn closed_envelope_flow_preserves_owned_headers_payload_and_actual_status() {
        let errors = check(&sources(), BINDING, &contract());
        assert!(errors.is_empty(), "{errors:#?}");
        let mut tuple = sources();
        tuple.insert(
            "src/types.rs".into(),
            TYPES.replace("Read { key: Payload }", "Read(Payload)"),
        );
        tuple.insert(
            "src/runtime.rs".into(),
            RUNTIME.replace("Effect::Read { key }", "Effect::Read(key)"),
        );
        assert!(check(&tuple, BINDING, &contract()).is_empty());
        let mut sync = sources();
        for source in sync.values_mut() {
            *source = source.replace("async fn", "fn").replace(".await", "");
        }
        sync.insert("src/runtime.rs".into(), "use crate::types::{Context, Effect, Request, ResultEnvelope}; use crate::io::Ports; use std::sync::Arc; fn dispatch(ports: Arc<Ports>, request: Request) -> ResultEnvelope { let (context, effect) = Context::split(request); match effect { Effect::Read { key } => crate::read::execute(ports, context, key), Effect::Stop => crate::stop::execute(ports, context) } }".into());
        let sync_binding = format!(
            "kind: synchronous-envelopes\n{}",
            BINDING.split("executors:\n").nth(1).unwrap()
        );
        let sync_binding = sync_binding.replacen("- {symbol:", "executors:\n- {symbol:", 1);
        let errors = check(&sync, &sync_binding, &contract());
        assert!(errors.is_empty(), "{errors:#?}");
        let mut invalid_sync = sync.clone();
        invalid_sync.insert(
            "src/io.rs".into(),
            invalid_sync["src/io.rs"].replace("fn read(", "async fn read("),
        );
        invalid_sync.insert(
            "src/read.rs".into(),
            invalid_sync["src/read.rs"]
                .replace("Ports::read(&ports, key)", "Ports::read(&ports, key).await"),
        );
        assert!(!check(&invalid_sync, &sync_binding, &contract()).is_empty());
    }
    #[test]
    fn format_capture_grammar_separates_references_from_prose_and_fill() {
        for (text, expected) in [
            ("observed {{observed}}", vec![]),
            ("{result:?}", vec!["result"]),
            ("{result:0observed$}", vec!["result", "observed"]),
            (
                "{result:width$.observed$}",
                vec!["result", "width", "observed"],
            ),
            ("{result:🦀>+#010.2x?}", vec!["result"]),
            ("{0:0$.1$}", vec![]),
            ("{result:.*}", vec!["result"]),
            ("{result   }", vec!["result"]),
        ] {
            let names = format_capture_names(text)
                .unwrap()
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>();
            assert_eq!(names, expected, "{text}");
        }
        for text in [
            "{",
            "}",
            "{result:.}",
            "{result:unknown}",
            "{nested{observed}}",
            "{_}",
        ] {
            assert!(format_capture_names(text).is_err(), "{text}");
        }
    }

    #[test]
    fn envelope_flow_macro_literals_are_not_callable_identifiers() {
        for body in [
            "panic!(\"long pressured stream ended before observed frame: {result:?}\")",
            "panic!(r###\"observed Context {result:?}\"###)",
            "format!(\"observed {{observed}} {result:width$.precision$}\")",
            "vec![\"{observed}\", r#\"Context observed\"#]",
            "assert_eq!(\"{observed}\", \"{observed}\", \"observed {result:?}\")",
            "write!(sink, \"observed {result:?}\")",
        ] {
            let mut source = sources();
            source
                .get_mut("src/lib.rs")
                .unwrap()
                .push_str(&format!("\nfn diagnostic() {{ {body}; }}\n"));
            assert!(check(&source, BINDING, &contract()).is_empty(), "{body}");
        }
    }

    #[test]
    fn envelope_flow_macro_identifiers_and_format_captures_remain_closed() {
        for body in [
            "panic!(\"{observed:?}\")",
            "panic!(r#\"{observed}\"#)",
            "panic!(\"{\\u{6f}bserved}\")",
            "format!(\"{result:0observed$}\")",
            "format!(\"{result:.observed$}\")",
            "assert!(true, \"{observed}\")",
            "write!(sink, \"{observed}\")",
            "vec![format!(\"{observed}\")]",
            "assert_eq!(r#observed, result)",
            "matches!(result, crate::types::Context { .. })",
            "vec![foreign::hidden!(\"plain prose\")]",
            "format!(concat!(\"{\", \"observed\", \"}\"))",
        ] {
            let mut source = sources();
            source
                .get_mut("src/lib.rs")
                .unwrap()
                .push_str(&format!("\nfn diagnostic() {{ {body}; }}\n"));
            let errors = check(&source, BINDING, &contract());
            assert!(
                errors.iter().any(|reason| reason.contains("src/lib.rs")
                    && reason.contains("macro `")
                    && reason.contains("token")),
                "{body}: {errors:?}"
            );
        }
        let mut source = sources();
        source
            .get_mut("src/lib.rs")
            .unwrap()
            .push_str("\nfn diagnostic() { panic!(\"{observed}\"); }\n");
        let errors = check(&source, BINDING, &contract());
        assert!(
            errors.iter().any(
                |reason| reason.contains("src/lib.rs: macro `panic!` at 2:19")
                    && reason.contains("capture `observed`")
            ),
            "{errors:?}"
        );
    }

    #[test]
    fn envelope_flow_accepts_documented_explicit_imports() {
        for attr in [
            "#[doc(hidden)]",
            "#[doc = \"Import documentation\"]",
            "/// Import documentation",
            "#[doc(alias = \"DocumentedPort\")]",
        ] {
            let mut sources = sources();
            sources.get_mut("src/lib.rs").unwrap().push_str(&format!(
                "\n{attr}\npub(crate) use crate::io::Ports;\n#[derive(Default)] struct DocumentationProbe;\n"));
            assert!(check(&sources, BINDING, &contract()).is_empty(), "{attr}");
        }
    }

    #[test]
    fn envelope_flow_import_refusals_keep_the_exact_import_scope_and_cause() {
        let mut module_sources = sources();
        module_sources
            .get_mut("src/io.rs")
            .unwrap()
            .push_str("\n#[cfg(test)] use std::sync::Arc;\n");
        let module_errors = check(&module_sources, BINDING, &contract());
        assert!(
            module_errors.iter().any(|reason| reason
                .contains("src/io.rs: file-level import `std::sync::Arc`")
                && reason.contains("cfg")),
            "{module_errors:?}"
        );
        for (attr, cause) in [
            ("#[cfg(test)]", "cfg"),
            ("#[cfg_attr(test, doc(hidden))]", "cfg_attr"),
            ("#[path = \"other.rs\"]", "path"),
            ("#[foreign::generate]", "foreign::generate"),
            ("#[derive(Clone)]", "derive"),
        ] {
            let mut sources = sources();
            sources.get_mut("src/lib.rs").unwrap().push_str(&format!(
                "\n{attr}\nuse crate::io::Ports;\n#[derive(Default)] struct DocumentationProbe;\n"
            ));
            let errors = check(&sources, BINDING, &contract());
            assert!(
                errors.iter().any(|reason| reason.contains("src/lib.rs")
                    && reason.contains("file-level import `crate::io::Ports`")
                    && reason.contains(cause)),
                "{errors:?}"
            );
            assert!(
                errors
                    .iter()
                    .all(|reason| !reason.contains("open imports prevent standard derive")),
                "{errors:?}"
            );
        }
        for import in [
            "use crate::io::*;",
            "use crate::io::Ports as Renamed;",
            "use crate::io::Ports; use crate::other::Ports;",
        ] {
            let mut sources = sources();
            sources.get_mut("src/lib.rs").unwrap().push_str(&format!(
                "\n{import}\n#[derive(Default)] struct DocumentationProbe;\n"
            ));
            let errors = check(&sources, BINDING, &contract());
            assert!(
                errors
                    .iter()
                    .any(|reason| reason.contains("src/lib.rs")
                        && reason.contains("file-level import")),
                "{errors:?}"
            );
        }
    }

    #[test]
    fn envelope_flow_refuses_forgery_aliases_copies_and_unproven_calls() {
        let cases = [
            ("src/types.rs", "effect_id: request.effect_id", "effect_id: request.correlation_id"),
            ("src/types.rs", "effect_id: self.effect_id", "effect_id: self.correlation_id"),
            ("src/types.rs", "causation_id: request.causation_id", "causation_id: None"),
            ("src/types.rs", "correlation_id: request.correlation_id", "correlation_id: request.correlation_id + 1"),
            ("src/types.rs", "}, request.effect)", "}, Effect::Stop)"),
            ("src/types.rs", "causation_id: self.causation_id, status", "causation_id: self.causation_id, status: Status::Stopped"),
            ("src/types.rs", "pub(crate) struct Context", "#[derive(Clone)] pub(crate) struct Context"),
            ("src/types.rs", "pub(crate) struct Context", "#[derive(Copy, Clone)] pub(crate) struct Context"),
            ("src/types.rs", "effect_id: u64, requester: &'static str, correlation_id: u64, causation_id: Option<u64>,", "pub effect_id: u64, requester: &'static str, correlation_id: u64, causation_id: Option<u64>,"),
            ("src/types.rs", "impl Context {", "impl Context { fn forge() -> Self { panic!() }"),
            ("src/types.rs", "fn split(request:", "fn split<Request>(request:"),
            ("src/types.rs", "fn observed(self,", "fn observed(&self,"),
            ("src/types.rs", "pub(crate) struct Context", "pub struct Context"),
            ("src/runtime.rs", "Context::split(request)", "Context::split(request.clone())"),
            ("src/runtime.rs", "let (context, effect)", "let (request, effect)"),
            ("src/runtime.rs", "Effect::Read { key }", "Effect::Read { key: context }"),
            ("src/runtime.rs", "Effect::Read { key }", "Effect::Read { key, .. }"),
            ("src/runtime.rs", "Effect::Stop =>", "_ =>"),
            ("src/runtime.rs", "Effect::Stop =>", "Effect::Stop if true =>"),
            ("src/runtime.rs", "execute(ports, context, key)", "execute(ports, context.clone(), key)"),
            ("src/runtime.rs", "execute(ports, context, key)", "execute(ports, context, 0)"),
            ("src/runtime.rs", "execute(ports, context, key)", "execute(ports, context, <Foreign>::key)"),
            ("src/runtime.rs", "execute(ports, context, key)", "execute(ports, effect, key)"),
            ("src/runtime.rs", "Effect::Stop => crate::stop", "Effect::Stop => crate::read"),
            ("src/runtime.rs", "match effect {", "let other = Context::split(request); match effect {"),
            ("src/runtime.rs", "crate::read::execute(ports, context, key).await", "{ crate::read::execute(ports, context, key).await; crate::read::execute(ports, context, key).await }"),
            ("src/runtime.rs", "Box::pin(async move", "Box::pin(async"),
            ("src/runtime.rs", "use crate::types::{Context,", "use crate::types::{Context as Foreign,"),
            ("src/read.rs", "context: Context", "context: &Context"),
            ("src/read.rs", "let status =", "let mut status ="),
            ("src/read.rs", "context.observed(status)", "context.observed(crate::types::Status::Stopped)"),
            ("src/read.rs", "crate::io::Ports::read(&ports, key).await", "crate::io::Ports::stop(&ports).await"),
            ("src/read.rs", "crate::io::Ports::read(&ports, key).await", "crate::types::Status::Stopped"),
            ("src/read.rs", "crate::io::Ports::read(&ports, key).await", "crate::io::Ports::read(&ports, 0).await"),
            ("src/read.rs", "crate::io::Ports::read(&ports, key).await", "crate::io::Ports::read(&ports, <Foreign>::key).await"),
            ("src/read.rs", "pub(crate) async fn execute", "pub async fn execute"),
            ("src/io.rs", "-> Status { Status::Read", "-> bool { Status::Read"),
            ("src/lib.rs", "mod types;", "use other::types; mod types;"),
            ("src/lib.rs", "mod types;", "#[path = \"other.rs\"] mod types;"),
            ("src/lib.rs", "mod types;", "mod types {}"),
        ];
        for (file, before, after) in cases {
            let mut sources = sources();
            assert!(
                sources[file].contains(before),
                "ineffective mutation: {before}"
            );
            sources.insert(file.into(), sources[file].replace(before, after));
            assert!(
                !check(&sources, BINDING, &contract()).is_empty(),
                "accepted forgery: {file}: {before} -> {after}"
            );
        }
        for extra in [
            "fn bypass(request: crate::types::Request, ports: std::sync::Arc<crate::io::Ports>) { let (context, effect) = crate::types::Context::split(request); crate::read::execute(ports, context, 0); }",
            "fn bypass() { let alias = crate::read::execute; }",
            "type Alias = crate::types::Context;",
            "fn shadow() { struct Context; }",
            "impl Clone for crate::types::Context { fn clone(&self) -> Self { panic!() } }",
            "macro_rules! hidden { () => { crate::read::execute() }; }",
            "fn hidden() { hidden!(); }",
            "fn hidden() { std::matches!(value, crate::types::Context { .. }); }",
            "use other::Context as Alias;",
            "extern crate other as std;",
            "use other as std; fn hidden() { std::matches!(1, 1); }",
            "use other::vec; fn hidden() { vec![]; }",
            "#[foreign::generate] struct Hidden;",
        ] {
            let mut sources = sources();
            sources.insert("src/extra.rs".into(), extra.into());
            sources.get_mut("src/lib.rs").unwrap().push_str(" mod extra;");
            assert!(!check(&sources, BINDING, &contract()).is_empty(), "accepted extra source: {extra}");
        }
    }
    #[test]
    fn envelope_flow_requires_canonical_evidence_and_preserves_default_rejection() {
        let sources = sources();
        let mut wrong = contract();
        wrong.result_fields.remove("correlation_id");
        assert!(!check(&sources, BINDING, &wrong).is_empty());
        wrong = contract();
        wrong
            .protocols
            .insert("src/read.rs#execute".into(), "Stop".into());
        assert!(!check(&sources, BINDING, &wrong).is_empty());
        for (before, after) in [
            (
                "context: src/types.rs#Context",
                "context: src/foreign.rs#Context",
            ),
            ("request_parameter: 1", "request_parameter: 9"),
            (
                "status_symbol: 'src/io.rs#Ports::read'",
                "status_symbol: 'src/foreign.rs#Ports::read'",
            ),
        ] {
            assert!(!check(&sources, &BINDING.replace(before, after), &contract()).is_empty());
        }
        let files = sources
            .iter()
            .map(|(file, source)| (file.clone(), syn::parse_file(source).unwrap()))
            .collect();
        let binding = serde_yaml::from_str(BINDING).unwrap();
        assert!(!Index { files: &files }
            .validate(
                &binding,
                "src/runtime.rs#drive",
                &contract().protocols.keys().cloned().collect::<Vec<_>>(),
                &types()
            )
            .is_empty());
        let legacy = BINDING.split("envelope_flow:").next().unwrap();
        let errors = check(&sources, legacy, &contract());
        assert!(
            errors
                .iter()
                .any(|reason| reason.contains("exact EffectEnvelope")),
            "{errors:#?}"
        );
    }

    #[test]
    fn closed_envelope_flow_fixture_compiles_without_copyable_context_or_payload() {
        let root = std::env::temp_dir().join(format!(
            "rms-envelope-flow-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(root.join("src")).unwrap();
        let mut sources = sources();
        // Names carry no proof authority. Use a different context name for
        // the native replay and leave std::task::Context unambiguous.
        for source in sources.values_mut() {
            *source = source.replace("Context", "NativeIdentity");
        }
        let binding = BINDING.replace("Context", "NativeIdentity");
        let runtime = sources["src/runtime.rs"].replace("fn dispatch(", "pub(crate) fn dispatch(");
        sources.insert("src/runtime.rs".into(), runtime);
        sources.get_mut("src/types.rs").unwrap().push_str(r#"
pub(crate) fn issued(bytes: Vec<u8>) -> Request {
    Request { effect_id: 17, requester: "native", correlation_id: 29,
        causation_id: Some(31), idempotency_key: Some(37), effect: Effect::Read { key: Payload(bytes) } }
}
pub(crate) fn inspected(result: ResultEnvelope) -> (u64, &'static str, u64, Option<u64>, Vec<u8>) {
    let bytes = match result.status { Status::Read(Payload(bytes)) => bytes, Status::Stopped => Vec::new() };
    (result.effect_id, result.requester, result.correlation_id, result.causation_id, bytes)
}
"#);
        sources.get_mut("src/lib.rs").unwrap().push_str(r#"
#[doc(hidden)]
#[doc = "Native replay retains its documentation attributes."]
use std::future::Future;
struct Wake;
impl std::task::Wake for Wake { fn wake(self: std::sync::Arc<Self>) {} }
fn main() {
    let result = 0usize;
    if result != 0 { panic!("long pressured stream ended before observed frame: {result:?}"); }
    let bytes = vec![1, 2, 3, 4];
    let allocation = bytes.as_ptr();
    let request = types::issued(bytes);
    let mut pending = runtime::dispatch(std::sync::Arc::new(io::Ports), request);
    let waker = std::task::Waker::from(std::sync::Arc::new(Wake));
    let mut cx = std::task::Context::from_waker(&waker);
    let result = match pending.as_mut().poll(&mut cx) { std::task::Poll::Ready(result) => result, std::task::Poll::Pending => panic!() };
    let (effect_id, requester, correlation_id, causation_id, bytes) = types::inspected(result);
    assert_eq!(effect_id, 17); assert_eq!(requester, "native"); assert_eq!(correlation_id, 29); assert_eq!(causation_id, Some(31));
    assert_eq!(bytes.as_ptr(), allocation); assert_eq!(bytes, vec![1, 2, 3, 4]);
}
"#);
        assert!(check(&sources, &binding, &contract()).is_empty());
        for (path, source) in sources {
            std::fs::write(root.join(path), source).unwrap();
        }
        let output = std::process::Command::new("rustc")
            .current_dir(&root)
            .args([
                "--edition=2021",
                "-A",
                "warnings",
                "src/lib.rs",
                "-o",
                "fixture",
            ])
            .output()
            .expect("native Rust compiler is required for envelope-flow proof");
        let run = output.status.success().then(|| {
            std::process::Command::new(root.join("fixture"))
                .output()
                .unwrap()
        });
        std::fs::remove_dir_all(&root).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            run.unwrap().status.success(),
            "native identity or zero-copy assertion failed"
        );
    }
}

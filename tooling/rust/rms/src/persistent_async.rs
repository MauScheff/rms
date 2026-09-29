//! Opt-in Rust storage/signature proof. This is not a scheduling or authority proof.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use syn::{FnArg, GenericArgument, Item, PathArguments, ReturnType, Type, TypeParamBound};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum ExecutionBinding {
    Synchronous {},
    PersistentAsync {
        runtime: String,
        state_field: String,
        records_field: String,
        pending_field: String,
        input_poll: String,
        executors: Vec<ExecutorBinding>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExecutorBinding {
    symbol: String,
    request_parameter: usize,
}

impl ExecutionBinding {
    pub(crate) fn is_persistent(&self) -> bool {
        matches!(self, Self::PersistentAsync { .. })
    }
}

pub(crate) struct Types<'a> {
    pub state: &'a str,
    pub input: &'a str,
    pub record: &'a str,
    pub effect: &'a str,
    pub result: &'a str,
    pub request_envelope: &'a str,
    pub result_envelope: &'a str,
}

/// Source-owned declarations only. Duplicated names and unsupported type forms fail closed.
pub(crate) struct Index<'a> {
    pub files: &'a BTreeMap<String, syn::File>,
}

impl<'a> Index<'a> {
    fn items(&self) -> impl Iterator<Item = (&str, &Item)> {
        self.files
            .iter()
            .flat_map(|(path, file)| file.items.iter().map(move |item| (path.as_str(), item)))
    }

    fn named_items(&self, name: &str) -> Vec<(&str, &Item)> {
        self.items()
            .filter(|(_, item)| match item {
                Item::Struct(item) => item.ident == name,
                Item::Enum(item) => item.ident == name,
                Item::Type(item) => item.ident == name,
                Item::Trait(item) => item.ident == name,
                Item::Mod(item) => item.ident == name,
                _ => false,
            })
            .collect()
    }

    fn resolve<'b>(&'b self, ty: &'b Type, seen: &mut BTreeSet<String>) -> Option<&'b Type> {
        if let Type::Path(path) = ty {
            if path.qself.is_some() || path.path.segments.len() != 1 {
                return Some(ty);
            }
            let segment = path.path.segments.first()?;
            let name = segment.ident.to_string();
            let matches = self.named_items(&name);
            if matches.len() > 1 {
                return None;
            }
            if let Some((_, Item::Type(alias))) = matches.first() {
                if !matches!(segment.arguments, PathArguments::None)
                    || !alias.generics.params.is_empty()
                    || !seen.insert(name)
                {
                    return None;
                }
                return self.resolve(&alias.ty, seen);
            }
        }
        Some(ty)
    }

    fn nominal(&self, ty: &Type, expected: &str) -> bool {
        if expected.is_empty() {
            return false;
        }
        matches!(self.resolve(ty, &mut BTreeSet::new()), Some(Type::Path(path))
            if path.qself.is_none() && path.path.segments.len() == 1
            && path.path.segments[0].ident == expected
            && matches!(path.path.segments[0].arguments, PathArguments::None)
            && self.named_items(expected).len() == 1)
    }

    fn standard_path(&self, path: &syn::Path, name: &str, qualified: &[&str]) -> bool {
        let spelling = path
            .segments
            .iter()
            .map(|part| part.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if spelling == name {
            if !self.named_items(name).is_empty() {
                return false;
            }
            let mut imports = BTreeSet::new();
            for (_, item) in self.items() {
                if let Item::Use(item) = item {
                    collect_imports(&item.tree, "", &mut imports);
                }
            }
            let matching = imports
                .iter()
                .filter(|path| path.rsplit("::").next() == Some(name))
                .collect::<Vec<_>>();
            return if matching.is_empty() {
                matches!(name, "Vec" | "Box" | "Option")
            } else {
                matching
                    .iter()
                    .all(|path| qualified.contains(&path.as_str()))
            };
        }
        qualified.contains(&spelling.as_str())
            && self
                .named_items(path.segments[0].ident.to_string().as_str())
                .is_empty()
    }

    fn unary<'b>(&'b self, ty: &'b Type, name: &str, paths: &[&str]) -> Option<&'b Type> {
        let Type::Path(path) = self.resolve(ty, &mut BTreeSet::new())? else {
            return None;
        };
        if !self.standard_path(&path.path, name, paths) {
            return None;
        }
        let PathArguments::AngleBracketed(args) = &path.path.segments.last()?.arguments else {
            return None;
        };
        if args.args.len() != 1 {
            return None;
        }
        match args.args.first()? {
            GenericArgument::Type(ty) => Some(ty),
            _ => None,
        }
    }

    fn future_output<'b>(&'b self, ty: &'b Type, depth: usize) -> Option<&'b Type> {
        if depth > 16 {
            return None;
        }
        let ty = self.resolve(ty, &mut BTreeSet::new())?;
        for (name, paths) in [
            ("Pin", &["std::pin::Pin", "core::pin::Pin"][..]),
            ("Box", &["std::boxed::Box", "alloc::boxed::Box"][..]),
        ] {
            if let Some(inner) = self.unary(ty, name, paths) {
                return self.future_output(inner, depth + 1);
            }
        }
        let bounds = match ty {
            Type::TraitObject(t) => &t.bounds,
            Type::ImplTrait(t) => &t.bounds,
            _ => return None,
        };
        let mut output = None;
        for bound in bounds {
            let TypeParamBound::Trait(bound) = bound else {
                continue;
            };
            if self.standard_path(
                &bound.path,
                "Future",
                &["std::future::Future", "core::future::Future"],
            ) {
                if output.is_some() {
                    return None;
                }
                let PathArguments::AngleBracketed(args) = &bound.path.segments.last()?.arguments
                else {
                    return None;
                };
                if args.args.len() != 1 {
                    return None;
                }
                let GenericArgument::AssocType(assoc) = args.args.first()? else {
                    return None;
                };
                if assoc.ident != "Output" || assoc.generics.is_some() {
                    return None;
                }
                output = Some(&assoc.ty);
            }
        }
        output
    }

    fn signature(&self, symbol: &str) -> Option<&syn::Signature> {
        let (path, name) = symbol
            .split_once('#')
            .map_or((None, symbol), |(path, name)| (Some(path), name));
        let mut found = Vec::new();
        for (file, item) in self
            .items()
            .filter(|(file, _)| path.is_none_or(|path| path == *file))
        {
            let _ = file;
            match item {
                Item::Fn(item) if item.sig.ident == name => found.push(&item.sig),
                Item::Trait(item) => {
                    for member in &item.items {
                        if let syn::TraitItem::Fn(method) = member {
                            if format!("{}::{}", item.ident, method.sig.ident) == name {
                                found.push(&method.sig);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        if found.len() == 1 {
            found.pop()
        } else {
            None
        }
    }

    pub(crate) fn validate(
        &self,
        binding: &ExecutionBinding,
        driver: &str,
        protocols: &[String],
        types: &Types<'_>,
    ) -> Vec<String> {
        let ExecutionBinding::PersistentAsync {
            runtime,
            state_field,
            records_field,
            pending_field,
            input_poll,
            executors,
        } = binding
        else {
            return Vec::new();
        };
        let mut errors = Vec::new();
        let Some((runtime_path, runtime_name)) = runtime.split_once('#') else {
            return vec!["runtime must name an exact path#type".into()];
        };
        let runtime_items = self.named_items(runtime_name);
        let runtime_struct = match runtime_items.as_slice() {
            [(path, Item::Struct(item))] if *path == runtime_path => Some(item),
            _ => None,
        };
        let field = |name: &str| {
            runtime_struct
                .and_then(|item| {
                    item.fields
                        .iter()
                        .find(|field| field.ident.as_ref().is_some_and(|id| id == name))
                })
                .map(|field| &field.ty)
        };
        if field(state_field).is_none_or(|ty| !self.nominal(ty, types.state)) {
            errors.push("runtime state field must have the exact declared State type".into());
        }
        if field(records_field)
            .and_then(|ty| self.unary(ty, "Vec", &["std::vec::Vec", "alloc::vec::Vec"]))
            .is_none_or(|ty| !self.nominal(ty, types.record))
        {
            errors.push("runtime records field must retain Vec<TransitionRecord>".into());
        }
        if field(pending_field)
            .and_then(|ty| self.unary(ty, "Vec", &["std::vec::Vec", "alloc::vec::Vec"]))
            .and_then(|ty| self.future_output(ty, 0))
            .is_none_or(|ty| !self.nominal(ty, types.result_envelope))
        {
            errors.push(
                "runtime pending field must retain Vec<Future<Output = EffectResultEnvelope>>"
                    .into(),
            );
        }
        let signature = self.signature(driver);
        let runtime_parameter = signature
            .and_then(|sig| sig.inputs.first())
            .and_then(parameter_type);
        if signature.is_none_or(|sig| sig.asyncness.is_none())
            || !matches!(runtime_parameter,
            Some(Type::Reference(reference)) if reference.mutability.is_some() && self.nominal(&reference.elem, runtime_name))
        {
            errors.push(
                "persistent driver must be async and accept &mut Runtime as its first parameter"
                    .into(),
            );
        }
        let poll = self.signature(input_poll);
        if poll.is_some_and(|signature| signature.asyncness.is_some())
            || !input_poll.contains('#')
            || poll
                .and_then(return_type)
                .and_then(|ty| self.unary(ty, "Poll", &["std::task::Poll", "core::task::Poll"]))
                .and_then(|ty| {
                    self.unary(
                        ty,
                        "Option",
                        &["std::option::Option", "core::option::Option"],
                    )
                })
                .is_none_or(|ty| !self.nominal(ty, types.input))
        {
            errors.push("exact input poll method must return Poll<Option<Input>>".into());
        }
        // Every protocol must remain bound. The new variant does not erase enum ownership.
        for name in [types.effect, types.result] {
            if !matches!(self.named_items(name).as_slice(), [(_, Item::Enum(_))]) {
                errors.push(format!("{name} must remain one declared closed enum"));
            }
        }
        for (envelope, payload) in [
            (types.request_envelope, types.effect),
            (types.result_envelope, types.result),
        ] {
            if !matches!(self.named_items(envelope).as_slice(), [(_, Item::Struct(item))]
                if item.fields.iter().any(|field| self.nominal(&field.ty, payload)))
            {
                errors.push(format!(
                    "{envelope} must retain its exact {payload} payload"
                ));
            }
        }
        let symbols = executors
            .iter()
            .map(|executor| executor.symbol.clone())
            .collect::<BTreeSet<_>>();
        if symbols.len() != executors.len() || symbols != protocols.iter().cloned().collect() {
            errors.push(
                "executor bindings must exactly cover the declared effect protocol symbols".into(),
            );
        }
        for executor in executors {
            let signature = self.signature(&executor.symbol);
            let request = signature
                .and_then(|sig| sig.inputs.iter().nth(executor.request_parameter))
                .and_then(parameter_type);
            if !executor.symbol.contains('#')
                || request.is_none_or(|ty| !self.nominal(ty, types.request_envelope))
            {
                errors.push(format!(
                    "{} request parameter must be the exact EffectEnvelope",
                    executor.symbol
                ));
            }
            let output = signature.and_then(return_type).and_then(|ty| {
                if signature.is_some_and(|sig| sig.asyncness.is_some()) {
                    Some(ty)
                } else {
                    self.future_output(ty, 0)
                }
            });
            if output.is_none_or(|ty| !self.nominal(ty, types.result_envelope)) {
                errors.push(format!(
                    "{} Future::Output must be the exact EffectResultEnvelope",
                    executor.symbol
                ));
            }
        }
        errors
    }
}

fn parameter_type(argument: &FnArg) -> Option<&Type> {
    match argument {
        FnArg::Typed(argument) => Some(&argument.ty),
        _ => None,
    }
}
fn return_type(signature: &syn::Signature) -> Option<&Type> {
    match &signature.output {
        ReturnType::Type(_, ty) => Some(ty),
        _ => None,
    }
}
fn collect_imports(tree: &syn::UseTree, prefix: &str, imports: &mut BTreeSet<String>) {
    match tree {
        syn::UseTree::Path(path) => {
            collect_imports(&path.tree, &format!("{prefix}{}::", path.ident), imports)
        }
        syn::UseTree::Name(name) => {
            imports.insert(format!("{prefix}{}", name.ident));
        }
        syn::UseTree::Rename(rename) => {
            imports.insert(format!("{prefix}{}::{}", rename.ident, rename.rename));
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_imports(item, prefix, imports);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"
use std::{future::Future, pin::Pin, task::Poll};
struct State;
enum Input { Result(ResultEnvelope) }
struct Record;
enum Effect { Start }
enum EffectResult { Started }
struct RequestEnvelope { effect: Effect }
struct ResultEnvelope { result: EffectResult }
type EffectFuture = Pin<Box<dyn Future<Output = ResultEnvelope> + Send>>;
struct Driver { state: State, records: Vec<Record>, pending: Vec<EffectFuture> }
trait Adapters { fn poll_input(&mut self) -> Poll<Option<Input>>; }
async fn drive(driver: &mut Driver, adapters: &mut impl Adapters) {}
fn execute(adapters: &mut impl Adapters, request: RequestEnvelope) -> EffectFuture { todo!() }
"#;

    fn binding() -> ExecutionBinding {
        serde_yaml::from_str(
            r#"
kind: persistent-async
runtime: src/driver.rs#Driver
state_field: state
records_field: records
pending_field: pending
input_poll: src/driver.rs#Adapters::poll_input
executors:
- symbol: src/driver.rs#execute
  request_parameter: 1
"#,
        )
        .unwrap()
    }

    fn check(source: &str) -> Vec<String> {
        let files = BTreeMap::from([("src/driver.rs".into(), syn::parse_file(source).unwrap())]);
        Index { files: &files }.validate(
            &binding(),
            "src/driver.rs#drive",
            &["src/driver.rs#execute".into()],
            &Types {
                state: "State",
                input: "Input",
                record: "Record",
                effect: "Effect",
                result: "EffectResult",
                request_envelope: "RequestEnvelope",
                result_envelope: "ResultEnvelope",
            },
        )
    }

    #[test]
    fn persistent_async_storage_and_future_output_are_checked() {
        assert!(check(SOURCE).is_empty());
        for (from, to, diagnostic) in [
            ("state: State", "state: Input", "state field"),
            (
                "records: Vec<Record>",
                "records: Vec<EffectResult>",
                "records field",
            ),
            (
                "pending: Vec<EffectFuture>",
                "pending: Option<EffectFuture>",
                "pending field",
            ),
            (
                "Output = ResultEnvelope",
                "Output = EffectResult",
                "Future::Output",
            ),
            ("driver: &mut Driver", "driver: &Driver", "&mut Runtime"),
            ("async fn drive", "fn drive", "must be async"),
            (
                "request: RequestEnvelope",
                "request: Effect",
                "request parameter",
            ),
            (
                "Poll<Option<Input>>",
                "Poll<Option<EffectResult>>",
                "input poll",
            ),
            (
                "result: EffectResult",
                "result: Effect",
                "exact EffectResult payload",
            ),
            (
                "enum EffectResult { Started }",
                "struct EffectResult;",
                "closed enum",
            ),
        ] {
            let errors = check(&SOURCE.replace(from, to));
            assert!(
                errors.iter().any(|error| error.contains(diagnostic)),
                "{from} -> {to}: {errors:?}"
            );
        }
    }

    #[test]
    fn persistent_async_rejects_cycles_shadowed_futures_and_ambiguous_types() {
        for source in [
            SOURCE.replace(
                "Pin<Box<dyn Future<Output = ResultEnvelope> + Send>>",
                "EffectFuture",
            ),
            format!("{SOURCE}\nstruct Driver;"),
            format!("{SOURCE}\ntrait Future {{ type Output; }}"),
            SOURCE.replace(
                "Pin<Box<dyn Future<Output = ResultEnvelope> + Send>>",
                "Option<ResultEnvelope>",
            ),
        ] {
            assert!(!check(&source).is_empty());
        }
    }

    #[test]
    fn persistent_async_supports_direct_async_executor_output() {
        let source = SOURCE.replace("fn execute(", "async fn execute(").replace(
            "request: RequestEnvelope) -> EffectFuture",
            "request: RequestEnvelope) -> ResultEnvelope",
        );
        assert!(check(&source).is_empty());
    }

    #[test]
    fn persistent_async_grammar_rejects_unknown_fields_and_variants() {
        assert!(serde_yaml::from_str::<ExecutionBinding>("kind: automatic").is_err());
        assert!(
            serde_yaml::from_str::<ExecutionBinding>("kind: synchronous\nallow_unknown: true")
                .is_err()
        );
        let binding: ExecutionBinding = serde_yaml::from_str("kind: synchronous").unwrap();
        assert!(!binding.is_persistent());
    }
}

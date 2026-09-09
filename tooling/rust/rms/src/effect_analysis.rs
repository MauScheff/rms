use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use syn::visit::{self, Visit};
use syn::{
    Expr, ExprCall, ExprMacro, ExprMethodCall, FnArg, ImplItemFn, ItemFn, ItemImpl, ItemMod, Local,
    Pat, Type, UseTree,
};
use tree_sitter::{Language, Node, Parser};

#[path = "rust_effect_types.rs"]
mod rust_effect_types;
use rust_effect_types::{RustTypeIndex, RustValueType};

pub(crate) const EFFECT_ANALYSIS_SPEC: &str = "rms/effect-analysis/v0.1";
pub(crate) const PURE_ALLOWLIST_VERSION: &str = "rms/pure-call-allowlist/v0.2";
pub(crate) const AUTHORITY_ROOT_VERSION: &str = "rms/authority-root-allowlist/v0.2";

#[derive(Clone, Debug)]
pub(crate) struct SemanticFunctionExpectation {
    pub(crate) id: String,
    pub(crate) symbol: String,
    pub(crate) purity: String,
    pub(crate) authorities: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct AuthorityFacade {
    pub(crate) authority: String,
    pub(crate) symbol: String,
}

#[derive(Clone, Debug)]
pub(crate) struct AnalysisInput {
    pub(crate) binding: String,
    pub(crate) source_digest: String,
    pub(crate) tool_digest: String,
    pub(crate) sources: BTreeMap<String, String>,
    pub(crate) semantic_functions: Vec<SemanticFunctionExpectation>,
    pub(crate) authority_facades: Vec<AuthorityFacade>,
    pub(crate) trusted_external_calls: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct EffectAnalysis {
    pub(crate) spec: &'static str,
    pub(crate) binding: String,
    pub(crate) source_digest: String,
    pub(crate) tool_digest: String,
    pub(crate) result: AnalysisResult,
    pub(crate) functions: Vec<FunctionAnalysis>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AnalysisResult {
    Pass,
    Fail,
    Unsupported,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct FunctionAnalysis {
    pub(crate) id: String,
    pub(crate) symbol: String,
    pub(crate) declared_purity: String,
    pub(crate) declared_authorities: Vec<String>,
    pub(crate) direct_calls: Vec<String>,
    pub(crate) resolved_callees: Vec<String>,
    pub(crate) direct_authorities: Vec<String>,
    pub(crate) transitive_authorities: Vec<String>,
    pub(crate) unresolved_calls: Vec<String>,
    pub(crate) verdict: FunctionVerdict,
    pub(crate) reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum FunctionVerdict {
    Pass,
    Fail,
    Unsupported,
}

#[derive(Clone, Debug, Default)]
struct FunctionNode {
    binding: String,
    path: String,
    name: String,
    qualified_name: String,
    callable_selector: Option<String>,
    calls: BTreeSet<String>,
    rust_unsafe_calls: BTreeSet<String>,
    direct_authorities: BTreeSet<String>,
    rust_regex_match_names: BTreeSet<String>,
    swift_standard_value_names: BTreeSet<String>,
}

pub(crate) fn analyze(input: AnalysisInput) -> EffectAnalysis {
    let supported = binding_is_supported(&input.binding)
        || (input.binding == "executable"
            && input
                .semantic_functions
                .iter()
                .all(|function| symbol_source_binding(&function.symbol).is_some()));
    if !supported {
        return EffectAnalysis {
            spec: EFFECT_ANALYSIS_SPEC,
            binding: input.binding,
            source_digest: input.source_digest,
            tool_digest: input.tool_digest,
            result: AnalysisResult::Unsupported,
            functions: input
                .semantic_functions
                .into_iter()
                .map(|function| FunctionAnalysis {
                    id: function.id,
                    symbol: function.symbol,
                    declared_purity: function.purity,
                    declared_authorities: function.authorities.into_iter().collect(),
                    direct_calls: Vec::new(),
                    resolved_callees: Vec::new(),
                    direct_authorities: Vec::new(),
                    transitive_authorities: Vec::new(),
                    unresolved_calls: Vec::new(),
                    verdict: FunctionVerdict::Unsupported,
                    reasons: vec!["binding has no effect analyzer".to_string()],
                })
                .collect(),
        };
    }

    let swift_sources = input
        .sources
        .iter()
        .filter(|(path, _)| {
            input.binding == "swift"
                || (input.binding == "executable" && source_binding(path) == Some("swift"))
        })
        .map(|(path, source)| (path.clone(), source.clone()))
        .collect::<BTreeMap<_, _>>();
    let swift_global_names = swift_global_standard_names(&swift_sources);
    let rust_types = RustTypeIndex::from_sources(&input.sources);
    let mut nodes = Vec::new();
    for (path, source) in &input.sources {
        let source_binding = if input.binding == "executable" {
            source_binding(path).unwrap_or("")
        } else {
            input.binding.as_str()
        };
        let mut extracted = if source_binding == "rust" {
            extract_rust_functions_with_types(path, source, &rust_types, &input.sources)
        } else {
            extract_tree_sitter_functions(source_binding, path, source, &swift_global_names)
        };
        if source_binding == "python" {
            refine_python_stdlib_calls(path, source, &input.sources, &mut extracted);
        }
        nodes.append(&mut extracted);
    }
    let facades = input
        .authority_facades
        .iter()
        .map(|facade| {
            (
                symbol_name(&facade.symbol).to_string(),
                facade.authority.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let authority_memberships = authority_memberships(&nodes, &input.authority_facades);
    let mut functions = Vec::new();
    for expectation in input.semantic_functions {
        functions.push(analyze_function(
            &expectation,
            &nodes,
            &facades,
            &authority_memberships,
            &input.trusted_external_calls,
        ));
    }
    functions.sort_by(|left, right| left.id.cmp(&right.id));
    let result = if functions
        .iter()
        .any(|function| function.verdict == FunctionVerdict::Fail)
    {
        AnalysisResult::Fail
    } else if functions
        .iter()
        .any(|function| function.verdict == FunctionVerdict::Unsupported)
    {
        AnalysisResult::Unsupported
    } else {
        AnalysisResult::Pass
    };
    EffectAnalysis {
        spec: EFFECT_ANALYSIS_SPEC,
        binding: input.binding,
        source_digest: input.source_digest,
        tool_digest: input.tool_digest,
        result,
        functions,
    }
}

fn binding_is_supported(binding: &str) -> bool {
    matches!(
        binding,
        "rust" | "swift" | "python" | "js" | "javascript" | "shell"
    )
}

fn symbol_source_binding(symbol: &str) -> Option<&'static str> {
    source_binding(symbol_path(symbol)?)
}

fn source_binding(path: &str) -> Option<&'static str> {
    let extension = std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())?;
    match extension {
        "rs" => Some("rust"),
        "swift" => Some("swift"),
        "py" => Some("python"),
        "js" | "mjs" | "cjs" | "ts" | "tsx" => Some("js"),
        "sh" | "bash" => Some("shell"),
        _ => None,
    }
}

fn analyze_function(
    expectation: &SemanticFunctionExpectation,
    nodes: &[FunctionNode],
    facades: &BTreeMap<String, String>,
    authority_memberships: &BTreeMap<usize, BTreeSet<String>>,
    trusted_external_calls: &BTreeSet<String>,
) -> FunctionAnalysis {
    let candidates = symbol_candidates(&expectation.symbol, nodes);
    if candidates.len() != 1 {
        return FunctionAnalysis {
            id: expectation.id.clone(),
            symbol: expectation.symbol.clone(),
            declared_purity: expectation.purity.clone(),
            declared_authorities: expectation.authorities.iter().cloned().collect(),
            direct_calls: Vec::new(),
            resolved_callees: Vec::new(),
            direct_authorities: Vec::new(),
            transitive_authorities: Vec::new(),
            unresolved_calls: Vec::new(),
            verdict: FunctionVerdict::Fail,
            reasons: vec![format!(
                "semantic symbol resolved to {} source functions; expected exactly one",
                candidates.len()
            )],
        };
    }

    let root = candidates[0];
    let direct_calls = nodes[root].calls.iter().cloned().collect::<Vec<_>>();
    let mut direct_authorities = authorities_for_node(root, nodes, facades);
    let mut transitive_authorities = BTreeSet::new();
    let mut resolved = BTreeSet::new();
    let mut unresolved = BTreeSet::new();
    let mut visited = BTreeSet::new();
    collect_closure(
        root,
        nodes,
        facades,
        &mut visited,
        &mut resolved,
        &mut unresolved,
        &mut transitive_authorities,
        trusted_external_calls,
    );
    let memberships = authority_memberships
        .get(&root)
        .cloned()
        .unwrap_or_default();
    if memberships.len() == 1 {
        let authority = memberships.iter().next().cloned().unwrap_or_default();
        direct_authorities = bind_ambient_authorities(direct_authorities, &authority);
        transitive_authorities = bind_ambient_authorities(transitive_authorities, &authority);
    }

    let mut reasons = Vec::new();
    if memberships.len() > 1 {
        reasons.push(format!(
            "semantic function is contained by multiple authority facades [{}]",
            memberships.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    if expectation.purity == "pure" {
        if !transitive_authorities.is_empty() {
            reasons.push(format!(
                "pure function reaches authorities [{}]",
                transitive_authorities
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !unresolved.is_empty() {
            reasons.push(format!(
                "pure function has unresolved calls [{}]",
                unresolved.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
        if !expectation.authorities.is_empty() {
            reasons.push("pure function must declare an empty authority row".to_string());
        }
    } else if expectation.purity == "effectful" {
        if expectation.authorities != transitive_authorities {
            reasons.push(format!(
                "declared authorities [{}] do not equal inferred authorities [{}]",
                expectation
                    .authorities
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", "),
                transitive_authorities
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if !unresolved.is_empty() {
            reasons.push(format!(
                "effectful function has unresolved calls [{}]",
                unresolved.iter().cloned().collect::<Vec<_>>().join(", ")
            ));
        }
    } else {
        reasons.push(format!("unsupported purity `{}`", expectation.purity));
    }
    FunctionAnalysis {
        id: expectation.id.clone(),
        symbol: expectation.symbol.clone(),
        declared_purity: expectation.purity.clone(),
        declared_authorities: expectation.authorities.iter().cloned().collect(),
        direct_calls,
        resolved_callees: resolved.into_iter().collect(),
        direct_authorities: direct_authorities.into_iter().collect(),
        transitive_authorities: transitive_authorities.into_iter().collect(),
        unresolved_calls: unresolved.into_iter().collect(),
        verdict: if reasons.is_empty() {
            FunctionVerdict::Pass
        } else {
            FunctionVerdict::Fail
        },
        reasons,
    }
}

fn symbol_candidates(symbol: &str, nodes: &[FunctionNode]) -> Vec<usize> {
    let expected_name = symbol_name(symbol);
    let expected_qualified = symbol_qualified_name(symbol);
    let expected_selector = symbol_callable_selector(symbol);
    let expected_path = symbol_path(symbol);
    let mut candidates = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.name == expected_name
                && expected_path.is_none_or(|path| normalized_path_matches(&node.path, path))
                && (!expected_qualified.contains("::") || node.qualified_name == expected_qualified)
                && expected_selector
                    .is_none_or(|selector| node.callable_selector.as_deref() == Some(selector))
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    if let Some(expected_path) = expected_path {
        let exact_path = candidates
            .iter()
            .copied()
            .filter(|index| {
                nodes[*index].path.replace('\\', "/") == expected_path.replace('\\', "/")
            })
            .collect::<Vec<_>>();
        if !exact_path.is_empty() {
            candidates = exact_path;
        }
    }
    if !expected_qualified.contains("::") {
        let free = candidates
            .iter()
            .copied()
            .filter(|index| nodes[*index].qualified_name == nodes[*index].name)
            .collect::<Vec<_>>();
        if free.len() == 1 {
            candidates = free;
        }
    }
    candidates
}

pub(crate) fn swift_symbol_resolves_exactly(source: &str, symbol: &str) -> bool {
    let path = symbol_path(symbol).unwrap_or("selector.swift");
    let nodes =
        extract_tree_sitter_functions("swift", path, source, &SwiftStandardNames::default());
    symbol_candidates(symbol, &nodes).len() == 1
}

fn authority_memberships(
    nodes: &[FunctionNode],
    facades: &[AuthorityFacade],
) -> BTreeMap<usize, BTreeSet<String>> {
    let mut memberships = BTreeMap::<usize, BTreeSet<String>>::new();
    for facade in facades {
        let roots = symbol_candidates(&facade.symbol, nodes);
        if roots.len() != 1 {
            continue;
        }
        let mut closure = BTreeSet::new();
        collect_local_members(roots[0], nodes, &mut closure);
        for index in closure {
            memberships
                .entry(index)
                .or_default()
                .insert(facade.authority.clone());
        }
    }
    memberships
}

fn collect_local_members(index: usize, nodes: &[FunctionNode], members: &mut BTreeSet<usize>) {
    if !members.insert(index) {
        return;
    }
    for call in &nodes[index].calls {
        let candidates = resolve_local_call(index, call, nodes);
        if candidates.len() == 1 {
            collect_local_members(candidates[0], nodes, members);
        }
    }
}

fn bind_ambient_authorities(
    authorities: BTreeSet<String>,
    facade_authority: &str,
) -> BTreeSet<String> {
    let has_ambient = authorities
        .iter()
        .any(|authority| authority != "dynamic-dispatch");
    let mut bound = authorities
        .into_iter()
        .filter(|authority| authority == "dynamic-dispatch")
        .collect::<BTreeSet<_>>();
    if has_ambient {
        bound.insert(facade_authority.to_string());
    }
    bound
}

#[allow(clippy::too_many_arguments)]
fn collect_closure(
    index: usize,
    nodes: &[FunctionNode],
    facades: &BTreeMap<String, String>,
    visited: &mut BTreeSet<usize>,
    resolved: &mut BTreeSet<String>,
    unresolved: &mut BTreeSet<String>,
    authorities: &mut BTreeSet<String>,
    trusted_external_calls: &BTreeSet<String>,
) {
    if !visited.insert(index) {
        return;
    }
    let node = &nodes[index];
    authorities.extend(authorities_for_node(index, nodes, facades));
    for call in &node.calls {
        if node.binding == "rust" && call.contains("().") && known_pure_call(call) {
            continue;
        }
        let candidates = resolve_local_call(index, call, nodes);
        if node.binding == "rust"
            && (candidates.len() == 1 || (call.contains('.') && !candidates.is_empty()))
        {
            for candidate in candidates {
                resolved.insert(nodes[candidate].qualified_name.clone());
                collect_closure(
                    candidate,
                    nodes,
                    facades,
                    visited,
                    resolved,
                    unresolved,
                    authorities,
                    trusted_external_calls,
                );
            }
            continue;
        }
        if node.binding == "shell" && candidates.len() == 1 {
            resolved.insert(nodes[candidates[0]].qualified_name.clone());
            collect_closure(
                candidates[0],
                nodes,
                facades,
                visited,
                resolved,
                unresolved,
                authorities,
                trusted_external_calls,
            );
            continue;
        }
        if let Some(authority) = authority_for_call(&node.binding, call)
            .or_else(|| facades.get(symbol_name(call)).cloned())
        {
            authorities.insert(authority);
            continue;
        }
        if node.rust_unsafe_calls.contains(call) {
            authorities.insert("unsafe".to_string());
            resolved.insert(call.clone());
            continue;
        }
        if trusted_external_calls.contains(call) {
            resolved.insert(call.clone());
            continue;
        }
        if known_pure_call(call)
            || rust_regex_match_offset_query(call, &node.rust_regex_match_names)
            || swift_standard_value_method(call, &node.swift_standard_value_names)
        {
            continue;
        }
        if candidates.len() == 1 {
            for candidate in candidates {
                resolved.insert(nodes[candidate].qualified_name.clone());
                collect_closure(
                    candidate,
                    nodes,
                    facades,
                    visited,
                    resolved,
                    unresolved,
                    authorities,
                    trusted_external_calls,
                );
            }
        } else if call_is_constructor(call) {
            continue;
        } else if node.binding == "shell" {
            unresolved.insert(call.clone());
        } else if call == "<dynamic-call>"
            || call.contains('.')
            || (call.contains("::")
                && call
                    .split("::")
                    .next()
                    .and_then(|root| root.chars().next())
                    .is_some_and(char::is_uppercase)
                && !call_is_constructor(call))
        {
            authorities.insert("dynamic-dispatch".to_string());
            resolved.insert(call.clone());
        } else {
            unresolved.insert(call.clone());
        }
    }
}

fn rust_regex_match_offset_query(call: &str, regex_match_names: &BTreeSet<String>) -> bool {
    if !matches!(symbol_name(call), "start" | "end") {
        return false;
    }
    call.rsplit_once('.')
        .map(|(receiver, _)| receiver.trim_end_matches("()"))
        .is_some_and(|receiver| regex_match_names.contains(receiver))
}

fn resolve_local_call(index: usize, call: &str, nodes: &[FunctionNode]) -> Vec<usize> {
    let name = symbol_name(call);
    let expected_path = symbol_path(call);
    if nodes[index].binding == "rust" && expected_path.is_none() {
        if let Some((root, _)) = call.split_once("::") {
            if root.chars().next().is_some_and(char::is_lowercase)
                && !matches!(root, "crate" | "self" | "super")
                && !nodes
                    .iter()
                    .any(|node| rust_source_module_name(&node.path) == Some(root))
            {
                // An unverified external path must not fall back to an unrelated
                // local callable with the same leaf name.
                return Vec::new();
            }
        }
    }
    if nodes[index].binding == "swift" && !call.contains(['.', ':', '#']) {
        let origin = |path: &str| {
            path.strip_prefix("dependencies/")
                .and_then(|rest| rest.split('/').next())
                .unwrap_or("")
                .to_string()
        };
        let local = nodes
            .iter()
            .enumerate()
            .filter(|(_, candidate)| {
                candidate.binding == "swift"
                    && candidate.name == name
                    && candidate.qualified_name == candidate.name
                    && origin(&candidate.path) == origin(&nodes[index].path)
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        if !local.is_empty() {
            return local;
        }
    }
    let same_file_free = nodes
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            candidate.path == nodes[index].path
                && !symbol_qualified_name(call).contains("::")
                && candidate.name == name
                && candidate.qualified_name == candidate.name
                && expected_path.is_none_or(|path| normalized_path_matches(&candidate.path, path))
        })
        .map(|(candidate, _)| candidate)
        .collect::<Vec<_>>();
    if same_file_free.len() == 1 {
        return same_file_free;
    }
    let direct = nodes
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            candidate.name == name
                && expected_path.is_none_or(|path| normalized_path_matches(&candidate.path, path))
        })
        .map(|(candidate, _)| candidate)
        .collect::<Vec<_>>();
    if direct.len() == 1 {
        return direct;
    }
    let requested_qualified = call.replace('.', "::");
    let exact = direct
        .iter()
        .copied()
        .filter(|candidate| {
            requested_qualified == nodes[*candidate].qualified_name
                || requested_qualified.ends_with(&format!("::{}", nodes[*candidate].qualified_name))
        })
        .collect::<Vec<_>>();
    if exact.len() == 1 {
        return exact;
    }
    let module_qualified = direct
        .iter()
        .copied()
        .filter(|candidate| rust_qualified_call_matches(&requested_qualified, &nodes[*candidate]))
        .collect::<Vec<_>>();
    if module_qualified.len() == 1 {
        return module_qualified;
    }
    let free = direct
        .iter()
        .copied()
        .filter(|candidate| nodes[*candidate].qualified_name == nodes[*candidate].name)
        .collect::<Vec<_>>();
    if free.len() == 1 {
        return free;
    }
    if let Some((module, _)) = call.rsplit_once("::") {
        let module = module.rsplit("::").next().unwrap_or(module);
        let qualified = direct
            .iter()
            .copied()
            .filter(|candidate| rust_source_module_name(&nodes[*candidate].path) == Some(module))
            .collect::<Vec<_>>();
        if qualified.len() == 1 {
            return qualified;
        }
    }
    if let Some((factory, _)) = call.split_once("().") {
        let factory_name = symbol_name(factory);
        let factories = nodes
            .iter()
            .enumerate()
            .filter(|(_, candidate)| candidate.name == factory_name)
            .map(|(candidate, _)| candidate)
            .collect::<Vec<_>>();
        if factories.len() == 1 {
            return factories;
        }
    }
    if call.contains('.') {
        return direct;
    }
    Vec::new()
}

fn rust_qualified_call_matches(requested: &str, candidate: &FunctionNode) -> bool {
    let suffix = format!("::{}", candidate.qualified_name);
    let Some(prefix) = requested.strip_suffix(&suffix) else {
        return false;
    };
    let requested_module = prefix.rsplit("::").next().unwrap_or(prefix);
    rust_source_module_name(&candidate.path) == Some(requested_module)
}

fn rust_source_module_name(path: &str) -> Option<&str> {
    let path = std::path::Path::new(path);
    let stem = path.file_stem()?.to_str()?;
    if stem == "mod" {
        path.parent()?.file_name()?.to_str()
    } else {
        Some(stem)
    }
}

fn authorities_for_node(
    index: usize,
    nodes: &[FunctionNode],
    facades: &BTreeMap<String, String>,
) -> BTreeSet<String> {
    let node = &nodes[index];
    let mut authorities = if node.binding == "shell" {
        BTreeSet::new()
    } else {
        node.direct_authorities.clone()
    };
    for call in &node.calls {
        let local_candidates = resolve_local_call(index, call, nodes);
        if (node.binding == "shell" && local_candidates.len() == 1)
            || (node.binding == "rust"
                && (local_candidates.len() == 1
                    || (call.contains('.') && !local_candidates.is_empty())))
        {
            continue;
        }
        if let Some(authority) = authority_for_call(&node.binding, call)
            .or_else(|| facades.get(symbol_name(call)).cloned())
        {
            authorities.insert(authority);
        }
    }
    authorities
}

fn authority_for_call(binding: &str, call: &str) -> Option<String> {
    if binding == "python" {
        match call {
            "python-stdlib.time.sleep" => return Some("clock".to_string()),
            "python-stdlib.shutil.copy2" | "python-stdlib.Path.stat" => {
                return Some("filesystem".to_string());
            }
            _ => {}
        }
    }
    let compact = call.replace(' ', "");
    let lower = compact.to_ascii_lowercase();
    let authority = if [
        "std::fs",
        "file::open",
        "read_to_string",
        "write_file",
        "readfile",
        "writefile",
        "pathlib.",
        "walkdir",
        "canonicalize",
        "create_dir",
        "read_dir",
        "remove_dir",
        "remove_file",
        "is_file",
        "is_dir",
        "file_type",
        "exists",
        "write_all",
        "sync_all",
        "make_executable",
        "shell.file_redirect",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
        || (binding == "python"
            && matches!(
                call_leaf(&lower),
                "open"
                    | "read"
                    | "read_bytes"
                    | "read_text"
                    | "write_bytes"
                    | "write_text"
                    | "mkdir"
                    | "resolve"
            ))
        || path_contains_segment(&lower, "fs")
    {
        "filesystem"
    } else if [
        "std::process",
        "subprocess",
        "os.system",
        "process.",
        "shutil.which",
        "spawn",
        "exec(",
        "child.",
        "std::io::stdin",
        "std::io::stdout",
        "std::io::stderr",
        "std::thread",
        "std::sync::mpsc",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        "process"
    } else if [
        "systemtime::now",
        "instant::now",
        "datetime.now",
        "date.now",
        "time.time",
        "thread::sleep",
        "duration_since",
        ".elapsed",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        "clock"
    } else if ["rand::", "math.random", "random.", "securerandom"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        "randomness"
    } else if [
        "std::env",
        "process.env",
        "os.environ",
        "getenv",
        "current_dir",
        "current_exe",
        "temp_dir",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        "environment"
    } else if [
        "fetch",
        "urlsession",
        "reqwest",
        "socket",
        "http.",
        "https.",
        "tcp",
        "listener.incoming",
        "stream.read",
        "stream.write",
        "stream.flush",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
    {
        "network"
    } else if ["git2", "git_command", "source_revision"]
        .iter()
        .any(|marker| lower.contains(marker))
    {
        "git"
    } else if binding == "shell"
        && matches!(
            lower.as_str(),
            "awk"
                | "bash"
                | "cat"
                | "cp"
                | "curl"
                | "cut"
                | "date"
                | "dd"
                | "find"
                | "git"
                | "grep"
                | "head"
                | "ln"
                | "mkdir"
                | "mktemp"
                | "mv"
                | "perl"
                | "python"
                | "python3"
                | "rm"
                | "sed"
                | "sh"
                | "sort"
                | "tail"
                | "tee"
                | "touch"
                | "tr"
                | "uniq"
                | "wget"
                | "xargs"
        )
    {
        "process"
    } else {
        return None;
    };
    Some(authority.to_string())
}

fn call_leaf(call: &str) -> &str {
    call.trim_end_matches("()")
        .rsplit(['.', ':'])
        .find(|segment| !segment.is_empty())
        .unwrap_or(call)
}

fn known_pure_call(call: &str) -> bool {
    let name = symbol_name(call).trim_end_matches('!');
    (name == "try_from"
        && [
            "u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize",
        ]
        .iter()
        .any(|primitive| call == format!("{primitive}::try_from")))
        || matches!(
            name,
            "add"
                | "all"
                | "and_then"
                | "any"
                | "append"
                | "as_array"
                | "as_bool"
                | "as_bytes"
                | "as_deref"
                | "as_i64"
                | "as_mapping"
                | "as_mapping_mut"
                | "as_mut"
                | "as_object"
                | "as_object_mut"
                | "as_os_str"
                | "as_ptr"
                | "as_ref"
                | "as_sequence"
                | "as_sequence_mut"
                | "as_slice"
                | "as_str"
                | "as_u64"
                | "at"
                | "bool"
                | "binary_search_by"
                | "borrow"
                | "borrow_mut"
                | "byte_range"
                | "bytes"
                | "chain"
                | "char_indices"
                | "chars"
                | "captures"
                | "checked_add"
                | "checked_mul"
                | "checked_neg"
                | "checked_pow"
                | "checked_sub"
                | "child_by_field_name"
                | "children"
                | "clamp"
                | "clear"
                | "clone"
                | "cloned"
                | "cmp"
                | "collect"
                | "components"
                | "contains"
                | "contains_key"
                | "copy_from_slice"
                | "context"
                | "copied"
                | "count"
                | "dedup"
                | "dedup_by"
                | "decode"
                | "default"
                | "dict"
                | "display"
                | "description"
                | "drop"
                | "emit"
                | "end"
                | "ends_with"
                | "endswith"
                | "enumerate"
                | "eq"
                | "entry"
                | "extend"
                | "extend_from_slice"
                | "extension"
                | "filter"
                | "filter_entry"
                | "filter_map"
                | "file_name"
                | "file_stem"
                | "find"
                | "find_map"
                | "first"
                | "flatten"
                | "flat_map"
                | "fold"
                | "freeze"
                | "format"
                | "from"
                | "from_iter"
                | "from_f64"
                | "from_millis"
                | "from_ref"
                | "from_secs"
                | "from_str_radix"
                | "from_utf8"
                | "from_utf8_lossy"
                | "align_of"
                | "size_of"
                | "catch_unwind"
                | "fullmatch"
                | "get"
                | "get_mut"
                | "get_or_init"
                | "get_or_insert"
                | "group"
                | "hexdigest"
                | "ip_address"
                | "insert"
                | "int"
                | "inspect"
                | "into"
                | "into_iter"
                | "into_bytes"
                | "into_keys"
                | "into_path"
                | "into_values"
                | "items"
                | "is_absolute"
                | "is_alphanumeric"
                | "is_ascii_alphabetic"
                | "is_ascii_alphanumeric"
                | "is_ascii_digit"
                | "is_ascii_lowercase"
                | "is_ascii_uppercase"
                | "is_ascii_whitespace"
                | "is_boolean"
                | "is_disjoint"
                | "is_empty"
                | "is_err"
                | "is_finite"
                | "is_i64"
                | "is_ident"
                | "is_mapping"
                | "is_match"
                | "is_multiple_of"
                | "is_none"
                | "is_none_or"
                | "is_ok"
                | "is_ok_and"
                | "is_sequence"
                | "is_some"
                | "is_some_and"
                | "is_string"
                | "is_null"
                | "is_object"
                | "is_u64"
                | "is_valid"
                | "is_whitespace"
                | "isInteger"
                | "isinstance"
                | "iter"
                | "iter_errors"
                | "iter_mut"
                | "join"
                | "key"
                | "keys"
                | "label"
                | "last"
                | "len"
                | "len_utf8"
                | "lines"
                | "list"
                | "loads"
                | "dumps"
                | "lower"
                | "map"
                | "map_err"
                | "map_or"
                | "map_or_else"
                | "match"
                | "match_indices"
                | "matches"
                | "max"
                | "min"
                | "min_by"
                | "min_by_key"
                | "named_child"
                | "named_children"
                | "new"
                | "next"
                | "next_back"
                | "ok"
                | "ok_or"
                | "ok_or_else"
                | "once"
                | "null"
                | "or"
                | "or_default"
                | "or_else"
                | "or_insert_with"
                | "parent"
                | "parse"
                | "path"
                | "peek"
                | "peekable"
                | "pointer"
                | "pop"
                | "pop_front"
                | "position"
                | "printf"
                | "push"
                | "push_back"
                | "push_str"
                | "range"
                | "remove"
                | "replace"
                | "replace_range"
                | "replacen"
                | "rpartition"
                | "rev"
                | "reverse"
                | "reversed"
                | "retain"
                | "return"
                | "root_node"
                | "rsplit"
                | "rsplit_once"
                | "saturating_add"
                | "saturating_mul"
                | "saturating_sub"
                | "search"
                | "set"
                | "set_language"
                | "sha256"
                | "shift"
                | "sort"
                | "sort_by"
                | "sort_by_key"
                | "sorted"
                | "split"
                | "split_at"
                | "split_last"
                | "split_once"
                | "splitlines"
                | "split_whitespace"
                | "skip"
                | "starts_with"
                | "startswith"
                | "strip"
                | "strip_prefix"
                | "strip_suffix"
                | "strings"
                | "str"
                | "structural_types"
                | "take"
                | "take_while"
                | "test"
                | "then"
                | "then_some"
                | "then_with"
                | "to_ascii_lowercase"
                | "to_digit"
                | "to_le_bytes"
                | "to_lowercase"
                | "toLowerCase"
                | "to_os_string"
                | "to_owned"
                | "to_path_buf"
                | "to_string"
                | "to_string_lossy"
                | "to_str"
                | "to_uppercase"
                | "to_vec"
                | "trim"
                | "trim_end_matches"
                | "trim_matches"
                | "trim_start"
                | "trim_start_matches"
                | "trimmingCharacters"
                | "truncate"
                | "transpose"
                | "tuple"
                | "union"
                | "unset"
                | "update"
                | "unwrap_or"
                | "unwrap_or_default"
                | "unwrap_or_else"
                | "utf8_text"
                | "urlsplit"
                | "urlsafe_b64decode"
                | "values"
                | "vec"
                | "visit_block"
                | "visit_file"
                | "walk"
                | "windows"
                | "with"
                | "with_capacity"
                | "with_context"
                | "with_extension"
                | "cast"
                | "wrapping_add"
                | "wrapping_mul"
                | "zip"
        )
        || call.starts_with("serde_json::to_")
        || call.starts_with("serde_json::from_")
        || call.starts_with("serde_yaml::to_")
        || call.starts_with("serde_yaml::from_")
        || call.starts_with("jsonschema::validator_for")
        || call.starts_with("syn::parse_file")
        || call.ends_with("::parse")
        || call.starts_with("std::mem::take")
        || call.starts_with("sha256_")
}

fn path_contains_segment(call: &str, expected: &str) -> bool {
    call.split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .any(|segment| segment == expected)
}

fn swift_standard_value_method(call: &str, standard_value_names: &BTreeSet<String>) -> bool {
    let method = symbol_name(call);
    if !matches!(
        method,
        "addingReportingOverflow"
            | "dropFirst"
            | "firstIndex"
            | "flatMap"
            | "formUnion"
            | "joined"
            | "removeAll"
            | "removeValue"
    ) {
        return false;
    }
    let Some((receiver, _)) = call.rsplit_once('.') else {
        return false;
    };
    let receiver = receiver
        .rsplit(['.', ':'])
        .find(|part| !part.is_empty())
        .unwrap_or(receiver);
    standard_value_names.contains(receiver)
}

fn call_is_constructor(call: &str) -> bool {
    call.starts_with('.')
        || (call.contains('.')
            && call
                .split('.')
                .next()
                .and_then(|root| root.chars().next())
                .is_some_and(char::is_uppercase))
        || symbol_name(call)
            .chars()
            .next()
            .is_some_and(char::is_uppercase)
        || matches!(symbol_name(call), "new" | "default")
}

fn symbol_name(symbol: &str) -> &str {
    let callable = symbol.rsplit_once('#').map_or(symbol, |(_, name)| {
        name.split_once('(').map_or(name, |(name, _)| name)
    });
    callable
        .rsplit([':', '.'])
        .find(|part| !part.is_empty())
        .unwrap_or(symbol)
        .trim_end_matches('!')
}

fn symbol_path(symbol: &str) -> Option<&str> {
    symbol.split_once('#').map(|(path, _)| path)
}

fn symbol_qualified_name(symbol: &str) -> String {
    symbol
        .split_once('#')
        .map_or(symbol, |(_, name)| {
            name.split_once('(').map_or(name, |(name, _)| name)
        })
        .replace('.', "::")
}

fn symbol_callable_selector(symbol: &str) -> Option<&str> {
    let callable = symbol.split_once('#').map_or(symbol, |(_, name)| name);
    callable.find('(').map(|start| &callable[start..])
}

fn normalized_path_matches(actual: &str, expected: &str) -> bool {
    actual
        .replace('\\', "/")
        .ends_with(&expected.replace('\\', "/"))
}

#[derive(Default)]
struct RustCallCollector {
    calls: BTreeSet<String>,
    unsafe_calls: BTreeSet<String>,
    authorities: BTreeSet<String>,
    dynamic_symbols: BTreeSet<String>,
    parameter_types: BTreeMap<String, String>,
    local_closures: BTreeSet<String>,
    regex_names: BTreeSet<String>,
    regex_match_names: BTreeSet<String>,
    unsafe_depth: usize,
    type_index: RustTypeIndex,
    value_types: BTreeMap<String, RustValueType>,
    helpers: BTreeMap<String, ItemFn>,
    static_callbacks: BTreeMap<String, String>,
    callback_bindings: BTreeMap<String, String>,
}

impl<'ast> Visit<'ast> for RustCallCollector {
    fn visit_expr_for_loop(&mut self, node: &'ast syn::ExprForLoop) {
        self.visit_expr(&node.expr);
        let prior_values = self.value_types.clone();
        let prior_types = self.parameter_types.clone();
        let prior_dynamic = self.dynamic_symbols.clone();
        let prior_callbacks = self.callback_bindings.clone();
        let element = match self
            .type_index
            .expression_type(&node.expr, &self.value_types)
        {
            Some(RustValueType::Sequence(element) | RustValueType::Iterator(element)) => {
                Some(*element)
            }
            _ => None,
        };
        let mut names = BTreeSet::new();
        collect_rust_pattern_identifiers(&node.pat, &mut names);
        for name in &names {
            if self.value_types.contains_key(name) || self.parameter_types.contains_key(name) {
                self.dynamic_symbols.insert(name.clone());
            }
            self.value_types
                .insert(name.clone(), RustValueType::Unknown);
            self.parameter_types.remove(name);
            self.callback_bindings.remove(name);
        }
        if let (Pat::Ident(name), Some(element)) = (node.pat.as_ref(), element) {
            if let Some(value_type) = element.name() {
                self.parameter_types
                    .insert(name.ident.to_string(), value_type.clone());
                self.dynamic_symbols.remove(&name.ident.to_string());
            }
            self.value_types.insert(name.ident.to_string(), element);
        }
        self.visit_block(&node.body);
        self.value_types = prior_values;
        self.parameter_types = prior_types;
        self.dynamic_symbols = prior_dynamic;
        self.callback_bindings = prior_callbacks;
    }

    fn visit_arm(&mut self, node: &'ast syn::Arm) {
        let prior_values = self.value_types.clone();
        let prior_types = self.parameter_types.clone();
        let prior_dynamic = self.dynamic_symbols.clone();
        let prior_callbacks = self.callback_bindings.clone();
        let mut names = BTreeSet::new();
        collect_rust_pattern_identifiers(&node.pat, &mut names);
        for name in names {
            if self.value_types.contains_key(&name) || self.parameter_types.contains_key(&name) {
                self.dynamic_symbols.insert(name.clone());
            }
            self.value_types
                .insert(name.clone(), RustValueType::Unknown);
            self.parameter_types.remove(&name);
            self.callback_bindings.remove(&name);
        }
        visit::visit_arm(self, node);
        self.value_types = prior_values;
        self.parameter_types = prior_types;
        self.dynamic_symbols = prior_dynamic;
        self.callback_bindings = prior_callbacks;
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let prior_values = self.value_types.clone();
        let prior_types = self.parameter_types.clone();
        let prior_dynamic = self.dynamic_symbols.clone();
        let prior_callbacks = self.callback_bindings.clone();
        let prior_closures = self.local_closures.clone();
        visit::visit_block(self, block);
        self.value_types = prior_values;
        self.parameter_types = prior_types;
        self.dynamic_symbols = prior_dynamic;
        self.callback_bindings = prior_callbacks;
        self.local_closures = prior_closures;
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let syn::Expr::Path(path) = node.func.as_ref() {
            let call = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            let leaf = symbol_name(&call);
            if let Some(callback) = self.callback_bindings.get(&call) {
                self.calls.insert(callback.clone());
                visit::visit_expr_call(self, node);
                return;
            }
            if let Some(helper) = self.helpers.get(&call) {
                let callback_parameters = rust_dynamic_parameters(helper.sig.inputs.iter());
                let bindings = helper
                    .sig
                    .inputs
                    .iter()
                    .zip(&node.args)
                    .filter_map(|(parameter, argument)| {
                        let FnArg::Typed(parameter) = parameter else {
                            return None;
                        };
                        let Pat::Ident(name) = parameter.pat.as_ref() else {
                            return None;
                        };
                        let name = name.ident.to_string();
                        if !callback_parameters.contains(&name) {
                            return None;
                        }
                        let Expr::Path(argument) = argument else {
                            return None;
                        };
                        let name_in_caller = argument.path.get_ident()?.to_string();
                        Some((name, self.static_callbacks.get(&name_in_caller)?.clone()))
                    })
                    .collect::<BTreeMap<_, _>>();
                if !bindings.is_empty()
                    && bindings.len() == callback_parameters.len()
                    && helper.sig.inputs.len() == node.args.len()
                    && rust_callbacks_only_called(&helper.block, &callback_parameters)
                {
                    let mut specialized = RustCallCollector {
                        dynamic_symbols: callback_parameters,
                        parameter_types: rust_parameter_types(helper.sig.inputs.iter()),
                        value_types: self.type_index.parameters(&helper.sig),
                        type_index: self.type_index.clone(),
                        callback_bindings: bindings,
                        ..RustCallCollector::default()
                    };
                    if helper.sig.unsafety.is_some() {
                        specialized.authorities.insert("unsafe".into());
                    }
                    specialized.visit_block(&helper.block);
                    self.calls.extend(specialized.calls);
                    self.unsafe_calls.extend(specialized.unsafe_calls);
                    self.authorities.extend(specialized.authorities);
                    for argument in &node.args {
                        self.visit_expr(argument);
                    }
                    return;
                }
            }
            if self.local_closures.contains(leaf) {
                visit::visit_expr_call(self, node);
                return;
            }
            if self.dynamic_symbols.contains(leaf) {
                self.calls.insert("<dynamic-call>".to_string());
                visit::visit_expr_call(self, node);
                return;
            }
            if self.unsafe_depth > 0 {
                self.unsafe_calls.insert(call.clone());
            }
            self.calls.insert(call);
        } else {
            self.calls.insert("<dynamic-call>".to_string());
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let receiver = rust_expr_label(&node.receiver);
        let inferred_receiver = self
            .type_index
            .expression_type(&node.receiver, &self.value_types);
        let inferred_name = inferred_receiver.as_ref().and_then(RustValueType::name);
        let typed_receiver = match node.receiver.as_ref() {
            Expr::Path(path) if path.path.segments.len() == 1 => self
                .parameter_types
                .get(&path.path.segments[0].ident.to_string()),
            _ => None,
        }
        .or(inferred_name);
        let call = if let Some(receiver_type) = typed_receiver {
            format!("{receiver_type}::{}", node.method)
        } else if receiver.is_empty() {
            node.method.to_string()
        } else {
            format!("{receiver}.{}", node.method)
        };
        if typed_receiver.is_none()
            && rust_expr_root_ident(&node.receiver)
                .is_some_and(|root| self.parameter_types.contains_key(&root))
            && !known_pure_call(&call)
        {
            self.calls.insert("<dynamic-call>".to_string());
            visit::visit_expr_method_call(self, node);
            return;
        }
        if rust_expr_root_ident(&node.receiver)
            .is_some_and(|root| self.dynamic_symbols.contains(&root))
            && !known_pure_call(&call)
        {
            self.calls.insert("<dynamic-call>".to_string());
            visit::visit_expr_method_call(self, node);
            return;
        }
        if self.unsafe_depth > 0 {
            self.unsafe_calls.insert(call.clone());
        }
        self.calls.insert(call);
        if matches!(node.method.to_string().as_str(), "map" | "filter_map") {
            for argument in &node.args {
                let Expr::Path(path) = argument else { continue };
                let callable = path
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                if self.dynamic_symbols.contains(symbol_name(&callable)) {
                    self.calls.insert("<dynamic-call>".to_string());
                } else {
                    self.calls.insert(callable);
                }
            }
        }
        if let Some(element) = inferred_receiver
            .as_ref()
            .and_then(RustValueType::element)
            .filter(|_| {
                matches!(
                    node.method.to_string().as_str(),
                    "map" | "filter" | "filter_map" | "any" | "all" | "find"
                )
            })
        {
            self.visit_expr(&node.receiver);
            for argument in &node.args {
                if let Expr::Closure(closure) = argument {
                    if closure.inputs.len() == 1 {
                        let prior_values = self.value_types.clone();
                        let prior_types = self.parameter_types.clone();
                        let prior_dynamic = self.dynamic_symbols.clone();
                        let mut names = BTreeSet::new();
                        collect_rust_pattern_identifiers(&closure.inputs[0], &mut names);
                        for name in &names {
                            self.value_types.remove(name);
                            self.parameter_types.remove(name);
                            self.dynamic_symbols.insert(name.clone());
                        }
                        if let Pat::Ident(ident) = &closure.inputs[0] {
                            let name = ident.ident.to_string();
                            self.value_types.insert(name.clone(), element.clone());
                            if let Some(type_name) = element.name() {
                                self.parameter_types.insert(name.clone(), type_name.clone());
                                self.dynamic_symbols.remove(&name);
                            }
                        }
                        self.visit_expr(&closure.body);
                        self.value_types = prior_values;
                        self.parameter_types = prior_types;
                        self.dynamic_symbols = prior_dynamic;
                        continue;
                    }
                }
                self.visit_expr(argument);
            }
        } else {
            visit::visit_expr_method_call(self, node);
        }
    }

    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.authorities.insert("unsafe".to_string());
        self.unsafe_depth += 1;
        visit::visit_block(self, &node.block);
        self.unsafe_depth -= 1;
    }

    fn visit_expr_closure(&mut self, node: &'ast syn::ExprClosure) {
        let prior_callbacks = self.callback_bindings.clone();
        let prior_values = self.value_types.clone();
        let prior_types = self.parameter_types.clone();
        let prior_dynamic = self.dynamic_symbols.clone();
        let mut names = BTreeSet::new();
        for input in &node.inputs {
            collect_rust_pattern_identifiers(input, &mut names);
        }
        for name in names {
            self.callback_bindings.remove(&name);
            self.value_types
                .insert(name.clone(), RustValueType::Unknown);
            self.parameter_types.remove(&name);
            self.dynamic_symbols.insert(name);
        }
        visit::visit_expr_closure(self, node);
        self.parameter_types = prior_types;
        self.dynamic_symbols = prior_dynamic;
        self.value_types = prior_values;
        self.callback_bindings = prior_callbacks;
    }

    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        let call = node
            .mac
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        if matches!(
            symbol_name(&call),
            "print" | "println" | "eprint" | "eprintln"
        ) {
            self.authorities.insert("process".to_string());
        }
        visit::visit_expr_macro(self, node);
    }

    fn visit_local(&mut self, node: &'ast Local) {
        let inferred = node.init.as_ref().and_then(|init| {
            self.type_index
                .expression_type(&init.expr, &self.value_types)
        });
        let mut bound_names = BTreeSet::new();
        collect_rust_pattern_identifiers(&node.pat, &mut bound_names);
        // Inspect the initializer in the old scope, then install the new binding.
        visit::visit_local(self, node);
        for name in &bound_names {
            self.callback_bindings.remove(name);
            let shadowed = self.value_types.contains_key(name)
                || self.parameter_types.contains_key(name)
                || self.dynamic_symbols.contains(name);
            self.value_types
                .insert(name.clone(), RustValueType::Unknown);
            self.parameter_types.remove(name);
            self.local_closures.remove(name);
            if shadowed {
                self.dynamic_symbols.insert(name.clone());
            }
        }
        if let (Pat::Ident(name), Some(value)) = (&node.pat, inferred) {
            if let Some(type_name) = value.name() {
                self.parameter_types
                    .insert(name.ident.to_string(), type_name.clone());
                self.dynamic_symbols.remove(&name.ident.to_string());
            }
            self.value_types.insert(name.ident.to_string(), value);
        }
        if node
            .init
            .as_ref()
            .is_some_and(|init| matches!(init.expr.as_ref(), Expr::Closure(_)))
        {
            if let Pat::Ident(ident) = &node.pat {
                self.local_closures.insert(ident.ident.to_string());
            }
        }
        if let Some(init) = node.init.as_ref() {
            if rust_expr_constructs_regex(&init.expr) {
                collect_rust_pattern_identifiers(&node.pat, &mut self.regex_names);
            } else if rust_expr_finds_regex_match(&init.expr, &self.regex_names) {
                collect_rust_pattern_identifiers(&node.pat, &mut self.regex_match_names);
            }
        }
    }
}

fn rust_expr_constructs_regex(expression: &Expr) -> bool {
    let Expr::Call(call) = expression else {
        return false;
    };
    let Expr::Path(path) = call.func.as_ref() else {
        return false;
    };
    let segments = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    segments.ends_with(&["Regex".to_string(), "new".to_string()])
}

fn rust_expr_finds_regex_match(expression: &Expr, regex_names: &BTreeSet<String>) -> bool {
    let Expr::MethodCall(call) = expression else {
        return false;
    };
    call.method == "find"
        && rust_expr_root_ident(&call.receiver).is_some_and(|name| regex_names.contains(&name))
}

fn collect_rust_pattern_identifiers(pattern: &Pat, names: &mut BTreeSet<String>) {
    match pattern {
        Pat::Ident(ident) => {
            names.insert(ident.ident.to_string());
        }
        Pat::TupleStruct(tuple) => {
            for element in &tuple.elems {
                collect_rust_pattern_identifiers(element, names);
            }
        }
        Pat::Tuple(tuple) => {
            for element in &tuple.elems {
                collect_rust_pattern_identifiers(element, names);
            }
        }
        Pat::Reference(reference) => collect_rust_pattern_identifiers(&reference.pat, names),
        Pat::Type(typed) => collect_rust_pattern_identifiers(&typed.pat, names),
        Pat::Paren(paren) => collect_rust_pattern_identifiers(&paren.pat, names),
        Pat::Struct(value) => {
            for field in &value.fields {
                collect_rust_pattern_identifiers(&field.pat, names);
            }
        }
        Pat::Slice(value) => {
            for element in &value.elems {
                collect_rust_pattern_identifiers(element, names);
            }
        }
        Pat::Or(value) => {
            for element in &value.cases {
                collect_rust_pattern_identifiers(element, names);
            }
        }
        _ => {}
    }
}

fn rust_expr_root_ident(expression: &Expr) -> Option<String> {
    match expression {
        Expr::Path(path) => path
            .path
            .segments
            .first()
            .map(|segment| segment.ident.to_string()),
        Expr::Call(call) => rust_expr_root_ident(&call.func),
        Expr::MethodCall(call) => rust_expr_root_ident(&call.receiver),
        Expr::Field(field) => rust_expr_root_ident(&field.base),
        Expr::Reference(reference) => rust_expr_root_ident(&reference.expr),
        _ => None,
    }
}

fn rust_dynamic_parameters<'a>(inputs: impl Iterator<Item = &'a FnArg>) -> BTreeSet<String> {
    inputs
        .filter_map(|argument| match argument {
            FnArg::Typed(argument) if rust_type_is_dynamic(argument.ty.as_ref()) => {
                match argument.pat.as_ref() {
                    Pat::Ident(ident) => Some(ident.ident.to_string()),
                    _ => None,
                }
            }
            FnArg::Typed(argument) => {
                let type_text = match argument.ty.as_ref() {
                    Type::Path(path) => path
                        .path
                        .segments
                        .last()
                        .map(|segment| segment.ident.to_string())
                        .unwrap_or_default(),
                    _ => String::new(),
                };
                if type_text.starts_with('F') {
                    match argument.pat.as_ref() {
                        Pat::Ident(ident) => Some(ident.ident.to_string()),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect()
}

fn rust_parameter_types<'a>(inputs: impl Iterator<Item = &'a FnArg>) -> BTreeMap<String, String> {
    inputs
        .filter_map(|argument| {
            let FnArg::Typed(argument) = argument else {
                return None;
            };
            let Pat::Ident(ident) = argument.pat.as_ref() else {
                return None;
            };
            let mut value = argument.ty.as_ref();
            while let Type::Reference(reference) = value {
                value = reference.elem.as_ref();
            }
            let Type::Path(path) = value else { return None };
            Some((
                ident.ident.to_string(),
                path.path.segments.last()?.ident.to_string(),
            ))
        })
        .collect()
}

fn rust_type_is_dynamic(value: &Type) -> bool {
    match value {
        Type::BareFn(_) | Type::ImplTrait(_) | Type::TraitObject(_) => true,
        Type::Reference(reference) => rust_type_is_dynamic(&reference.elem),
        Type::Paren(paren) => rust_type_is_dynamic(&paren.elem),
        Type::Group(group) => rust_type_is_dynamic(&group.elem),
        Type::Path(path) => path.path.segments.iter().any(|segment| {
            if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                arguments.args.iter().any(|argument| {
                    matches!(argument, syn::GenericArgument::Type(value) if rust_type_is_dynamic(value))
                })
            } else {
                false
            }
        }),
        _ => false,
    }
}

fn rust_expr_label(expression: &Expr) -> String {
    match expression {
        Expr::Path(path) => path
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>()
            .join("::"),
        Expr::Call(call) => match call.func.as_ref() {
            Expr::Path(path) => format!(
                "{}()",
                path.path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::")
            ),
            _ => String::new(),
        },
        Expr::MethodCall(call) => {
            let receiver = rust_expr_label(&call.receiver);
            if receiver.is_empty() {
                call.method.to_string()
            } else {
                format!("{receiver}.{}()", call.method)
            }
        }
        Expr::Try(value) => rust_expr_label(&value.expr),
        Expr::Await(value) => rust_expr_label(&value.base),
        Expr::Field(field) => {
            let receiver = rust_expr_label(&field.base);
            if receiver.is_empty() {
                String::new()
            } else {
                format!("{receiver}.field")
            }
        }
        _ => String::new(),
    }
}

#[derive(Default)]
struct RustFunctionCollector {
    nodes: Vec<FunctionNode>,
    path: String,
    owner: Vec<String>,
    test_depth: usize,
    type_index: RustTypeIndex,
    helpers: BTreeMap<String, ItemFn>,
    static_callbacks: BTreeMap<String, String>,
}

impl<'ast> Visit<'ast> for RustFunctionCollector {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if self.test_depth > 0 || has_test_attribute(&node.attrs) {
            return;
        }
        let mut calls = RustCallCollector {
            helpers: self
                .helpers
                .iter()
                .filter(|(name, _)| {
                    rust_unshadowed_callbacks(node, &self.static_callbacks).contains_key(*name)
                })
                .map(|(name, helper)| (name.clone(), helper.clone()))
                .collect(),
            static_callbacks: rust_unshadowed_callbacks(node, &self.static_callbacks),
            type_index: self.type_index.clone(),
            value_types: self.type_index.parameters(&node.sig),
            dynamic_symbols: rust_dynamic_parameters(node.sig.inputs.iter()),
            parameter_types: rust_parameter_types(node.sig.inputs.iter()),
            ..RustCallCollector::default()
        };
        if node.sig.unsafety.is_some() {
            calls.authorities.insert("unsafe".to_string());
        }
        calls.visit_block(&node.block);
        let name = node.sig.ident.to_string();
        self.nodes.push(FunctionNode {
            binding: "rust".to_string(),
            path: self.path.clone(),
            qualified_name: qualified_rust_name(&self.owner, &name),
            callable_selector: None,
            name,
            calls: calls.calls,
            rust_unsafe_calls: calls.unsafe_calls,
            direct_authorities: calls.authorities,
            rust_regex_match_names: calls.regex_match_names,
            swift_standard_value_names: BTreeSet::new(),
        });
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        if self.test_depth > 0 || has_test_attribute(&node.attrs) {
            return;
        }
        let mut calls = RustCallCollector {
            type_index: self.type_index.clone(),
            value_types: self.type_index.parameters(&node.sig),
            dynamic_symbols: rust_dynamic_parameters(node.sig.inputs.iter()),
            parameter_types: rust_parameter_types(node.sig.inputs.iter()),
            ..RustCallCollector::default()
        };
        if node.sig.unsafety.is_some() {
            calls.authorities.insert("unsafe".to_string());
        }
        calls.visit_block(&node.block);
        let name = node.sig.ident.to_string();
        self.nodes.push(FunctionNode {
            binding: "rust".to_string(),
            path: self.path.clone(),
            qualified_name: qualified_rust_name(&self.owner, &name),
            callable_selector: None,
            name,
            calls: calls.calls,
            rust_unsafe_calls: calls.unsafe_calls,
            direct_authorities: calls.authorities,
            rust_regex_match_names: calls.regex_match_names,
            swift_standard_value_names: BTreeSet::new(),
        });
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let owner = match node.self_ty.as_ref() {
            Type::Path(path) => path
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string()),
            _ => None,
        };
        if let Some(owner) = owner {
            self.owner.push(owner);
            syn::visit::visit_item_impl(self, node);
            self.owner.pop();
        } else {
            syn::visit::visit_item_impl(self, node);
        }
    }

    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        let is_test = has_test_attribute(&node.attrs) || node.ident == "tests";
        if is_test {
            self.test_depth += 1;
        }
        syn::visit::visit_item_mod(self, node);
        if is_test {
            self.test_depth -= 1;
        }
    }
}

fn has_test_attribute(attributes: &[syn::Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("test")
            || (attribute.path().is_ident("cfg")
                && attribute
                    .parse_args::<syn::Ident>()
                    .is_ok_and(|ident| ident == "test"))
    })
}

fn rust_unshadowed_callbacks(
    function: &ItemFn,
    callbacks: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    #[derive(Default)]
    struct Names(BTreeSet<String>);
    impl<'ast> Visit<'ast> for Names {
        fn visit_pat_ident(&mut self, node: &'ast syn::PatIdent) {
            self.0.insert(node.ident.to_string());
        }
        fn visit_item_fn(&mut self, node: &'ast ItemFn) {
            self.0.insert(node.sig.ident.to_string());
        }
        fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
            let mut aliases = BTreeMap::new();
            collect_rust_use_aliases(&node.tree, Vec::new(), &mut aliases);
            self.0.extend(aliases.into_keys());
        }
    }
    let mut names = Names::default();
    names.visit_signature(&function.sig);
    names.visit_block(&function.block);
    callbacks
        .iter()
        .filter(|(name, _)| !names.0.contains(*name))
        .map(|(name, target)| (name.clone(), target.clone()))
        .collect()
}

fn rust_callbacks_only_called(block: &syn::Block, names: &BTreeSet<String>) -> bool {
    struct Uses<'a> {
        names: &'a BTreeSet<String>,
        escaped: bool,
    }
    impl<'ast> Visit<'ast> for Uses<'_> {
        fn visit_expr_call(&mut self, call: &'ast ExprCall) {
            if matches!(call.func.as_ref(), Expr::Path(path) if path.path.get_ident().is_some_and(|name| self.names.contains(&name.to_string())))
            {
                for argument in &call.args {
                    self.visit_expr(argument);
                }
            } else {
                visit::visit_expr_call(self, call);
            }
        }
        fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
            if path
                .path
                .get_ident()
                .is_some_and(|name| self.names.contains(&name.to_string()))
            {
                self.escaped = true;
            }
        }
        fn visit_expr_macro(&mut self, _: &'ast ExprMacro) {
            self.escaped = true;
        }
    }
    let mut uses = Uses {
        names,
        escaped: false,
    };
    uses.visit_block(block);
    !uses.escaped
}

fn qualified_rust_name(owner: &[String], name: &str) -> String {
    if owner.is_empty() {
        name.to_string()
    } else {
        format!("{}::{name}", owner.join("::"))
    }
}

#[cfg(test)]
fn extract_rust_functions(path: &str, source: &str) -> Vec<FunctionNode> {
    extract_rust_functions_with_types(path, source, &RustTypeIndex::default(), &BTreeMap::new())
}

fn extract_rust_functions_with_types(
    path: &str,
    source: &str,
    types: &RustTypeIndex,
    sources: &BTreeMap<String, String>,
) -> Vec<FunctionNode> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut function_counts = BTreeMap::<String, usize>::new();
    for item in &file.items {
        if let syn::Item::Fn(function) = item {
            *function_counts
                .entry(function.sig.ident.to_string())
                .or_default() += 1;
        }
    }
    let mut collector = RustFunctionCollector {
        nodes: Vec::new(),
        path: path.to_string(),
        owner: Vec::new(),
        test_depth: 0,
        type_index: types.for_path(path),
        helpers: file
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function)
                    if matches!(function.vis, syn::Visibility::Inherited)
                        && function.attrs.is_empty()
                        && function_counts.get(&function.sig.ident.to_string()) == Some(&1) =>
                {
                    Some((function.sig.ident.to_string(), function.clone()))
                }
                _ => None,
            })
            .collect(),
        static_callbacks: file
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Fn(function)
                    if function.attrs.is_empty()
                        && function_counts.get(&function.sig.ident.to_string()) == Some(&1) =>
                {
                    Some((
                        function.sig.ident.to_string(),
                        format!("{path}#{}", function.sig.ident),
                    ))
                }
                _ => None,
            })
            .collect(),
    };
    let mut aliases = rust_import_aliases(&file);
    let safe_aliases = rust_unique_unconditional_imports(&file);
    for (name, target) in &mut aliases {
        if !safe_aliases.contains(name) || function_counts.contains_key(name) {
            continue;
        }
        if let Some(exact) = rust_exact_function_reference(path, target, sources, 0) {
            *target = exact;
        }
    }
    collector.static_callbacks.extend(
        aliases
            .iter()
            .filter(|(_, target)| target.contains('#'))
            .map(|(name, target)| (name.clone(), target.clone())),
    );
    collector.visit_file(&file);
    for node in &mut collector.nodes {
        node.calls = node
            .calls
            .iter()
            .map(|call| {
                let resolved = resolve_call_alias(call, &aliases);
                rust_exact_function_reference(path, &resolved, sources, 0).unwrap_or(resolved)
            })
            .collect();
        node.rust_unsafe_calls = node
            .rust_unsafe_calls
            .iter()
            .map(|call| resolve_call_alias(call, &aliases))
            .collect();
    }
    collector.nodes
}

fn rust_unique_unconditional_imports(file: &syn::File) -> BTreeSet<String> {
    let mut counts = BTreeMap::<String, usize>::new();
    let mut conditional = BTreeSet::new();
    for item in &file.items {
        if let syn::Item::Use(item) = item {
            let mut aliases = BTreeMap::new();
            collect_rust_use_aliases(&item.tree, Vec::new(), &mut aliases);
            for name in aliases.into_keys() {
                *counts.entry(name.clone()).or_default() += 1;
                if !item.attrs.is_empty() {
                    conditional.insert(name);
                }
            }
        }
    }
    counts
        .into_iter()
        .filter(|(name, count)| *count == 1 && !conditional.contains(name))
        .map(|(name, _)| name)
        .collect()
}

fn rust_exact_function_reference(
    path: &str,
    reference: &str,
    sources: &BTreeMap<String, String>,
    depth: usize,
) -> Option<String> {
    let exact = rust_exact_item_reference(path, reference, sources, depth)?;
    let (path, name) = exact.split_once('#')?;
    let file = syn::parse_file(sources.get(path)?).ok()?;
    file.items
        .iter()
        .any(|item| matches!(item, syn::Item::Fn(function) if function.sig.ident == name))
        .then_some(exact)
}

fn rust_exact_item_reference(
    path: &str,
    reference: &str,
    sources: &BTreeMap<String, String>,
    depth: usize,
) -> Option<String> {
    if depth > 8 {
        return None;
    }
    let parts = reference.split("::").collect::<Vec<_>>();
    if parts.len() > 1 {
        let prefix = if parts[0] == "crate" {
            path.split_once("/src/")
                .map(|(prefix, _)| prefix.to_string())
                .unwrap_or_default()
        } else {
            sources
                .get(&format!("rms-metadata/rust-crate-alias/{}", parts[0]))?
                .clone()
        };
        let root = if prefix.is_empty() {
            "src".to_string()
        } else {
            format!("{prefix}/src")
        };
        let target = if parts.len() == 2 {
            format!("{root}/lib.rs")
        } else {
            format!("{root}/{}.rs", parts[1..parts.len() - 1].join("/"))
        };
        return rust_exact_item_reference(&target, parts.last()?, sources, depth + 1);
    }
    let file = syn::parse_file(sources.get(path)?).ok()?;
    let functions = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Fn(function)
                if function.sig.ident == reference && function.attrs.is_empty() =>
            {
                Some(())
            }
            syn::Item::Struct(item) if item.ident == reference => Some(()),
            syn::Item::Enum(item) if item.ident == reference => Some(()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if functions.len() == 1 {
        return Some(format!("{path}#{reference}"));
    }
    if !functions.is_empty() {
        return None;
    }
    let mut targets = Vec::new();
    for item in &file.items {
        if let syn::Item::Use(item) = item {
            if !item.attrs.is_empty() {
                continue;
            }
            let mut aliases = BTreeMap::new();
            collect_rust_use_aliases(&item.tree, Vec::new(), &mut aliases);
            if let Some(target) = aliases.get(reference) {
                targets.push(target.clone());
            }
        }
    }
    if targets.is_empty() {
        fn glob_modules(tree: &UseTree, prefix: Vec<String>, modules: &mut Vec<String>) {
            match tree {
                UseTree::Path(item) => {
                    let mut prefix = prefix;
                    prefix.push(item.ident.to_string());
                    glob_modules(&item.tree, prefix, modules);
                }
                UseTree::Group(group) => {
                    for item in &group.items {
                        glob_modules(item, prefix.clone(), modules);
                    }
                }
                UseTree::Glob(_) => modules.push(prefix.join("::")),
                _ => {}
            }
        }
        let mut modules = Vec::new();
        for item in &file.items {
            if let syn::Item::Use(item) = item {
                if !item.attrs.is_empty() {
                    continue;
                }
                glob_modules(&item.tree, Vec::new(), &mut modules);
            }
        }
        let mut resolved = BTreeSet::new();
        for module in modules {
            // Expand only inspectable crate-local globs. Unknown external globs
            // cannot establish absence or a unique exported callable.
            let module_path = module.strip_prefix("crate::")?.replace("::", "/");
            let prefix = path
                .split_once("/src/")
                .map(|(prefix, _)| format!("{prefix}/"))
                .unwrap_or_default();
            if !sources.contains_key(&format!("{prefix}src/{module_path}.rs")) {
                return None;
            }
            if let Some(target) = rust_exact_item_reference(
                path,
                &format!("{module}::{reference}"),
                sources,
                depth + 1,
            ) {
                resolved.insert(target);
            }
        }
        return (resolved.len() == 1)
            .then(|| resolved.into_iter().next())
            .flatten();
    }
    if targets.len() != 1 {
        return None;
    }
    rust_exact_item_reference(path, &targets[0], sources, depth + 1)
}

fn rust_import_aliases(file: &syn::File) -> BTreeMap<String, String> {
    let mut aliases = BTreeMap::new();
    for item in &file.items {
        if let syn::Item::Use(item) = item {
            collect_rust_use_aliases(&item.tree, Vec::new(), &mut aliases);
        }
    }
    aliases
}

fn collect_rust_use_aliases(
    tree: &UseTree,
    prefix: Vec<String>,
    aliases: &mut BTreeMap<String, String>,
) {
    match tree {
        UseTree::Path(path) => {
            let mut prefix = prefix;
            prefix.push(path.ident.to_string());
            collect_rust_use_aliases(&path.tree, prefix, aliases);
        }
        UseTree::Name(name) => {
            let mut target = prefix;
            target.push(name.ident.to_string());
            aliases.insert(name.ident.to_string(), target.join("::"));
        }
        UseTree::Rename(rename) => {
            let mut target = prefix;
            target.push(rename.ident.to_string());
            aliases.insert(rename.rename.to_string(), target.join("::"));
        }
        UseTree::Group(group) => {
            for item in &group.items {
                collect_rust_use_aliases(item, prefix.clone(), aliases);
            }
        }
        UseTree::Glob(_) => {}
    }
}

fn resolve_call_alias(call: &str, aliases: &BTreeMap<String, String>) -> String {
    let leaf = call
        .split([':', '.', '('])
        .find(|part| !part.is_empty())
        .unwrap_or(call);
    let Some(target) = aliases.get(leaf) else {
        return call.to_string();
    };
    call.replacen(leaf, target, 1)
}

#[derive(Default)]
struct SwiftStandardNames {
    subscript: BTreeSet<String>,
    value: BTreeSet<String>,
}

fn extract_tree_sitter_functions(
    binding: &str,
    path: &str,
    source: &str,
    swift_global_names: &SwiftStandardNames,
) -> Vec<FunctionNode> {
    if path.contains("/tests/") || path.contains(".test.") {
        return Vec::new();
    }
    let Some(language) = tree_sitter_language(binding) else {
        return Vec::new();
    };
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return Vec::new();
    }
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    let aliases = tree_sitter_import_aliases(binding, path, tree.root_node(), source);
    let static_dispatches = if binding == "python" {
        python_static_dispatches(tree.root_node(), source, &aliases)
    } else {
        BTreeMap::new()
    };
    let mut function_nodes = Vec::new();
    collect_function_nodes(tree.root_node(), &mut function_nodes);
    function_nodes
        .into_iter()
        .filter_map(|node| {
            let name = function_node_name(node, source)?;
            let qualified_name = tree_sitter_qualified_name(node, source, &name);
            let swift_collection_names = if binding == "swift" {
                let mut names = swift_standard_collection_names(tree.root_node(), node, source);
                names.extend(swift_global_names.subscript.iter().cloned());
                names
            } else {
                BTreeSet::new()
            };
            let swift_standard_value_names = if binding == "swift" {
                let mut names = swift_standard_value_names(
                    tree.root_node(),
                    node,
                    source,
                    &swift_collection_names,
                );
                names.extend(swift_global_names.value.iter().cloned());
                names
            } else {
                BTreeSet::new()
            };
            let mut calls = BTreeSet::new();
            if binding == "shell" {
                collect_shell_call_nodes(node, source, &mut calls, true);
            } else {
                collect_call_nodes(
                    binding,
                    node,
                    source,
                    &swift_collection_names,
                    &mut calls,
                    true,
                );
            }
            let calls = calls
                .into_iter()
                .flat_map(|call| {
                    let dispatch_name = call.split_once('[').map(|(name, _)| name);
                    static_dispatches
                        .get(dispatch_name.unwrap_or_default())
                        .cloned()
                        .unwrap_or_else(|| vec![resolve_call_alias(&call, &aliases)])
                })
                .collect::<BTreeSet<_>>();
            let direct_authorities = calls
                .iter()
                .filter_map(|call| authority_for_call(binding, call))
                .collect();
            Some(FunctionNode {
                binding: binding.to_string(),
                path: path.to_string(),
                name,
                qualified_name,
                callable_selector: (binding == "swift")
                    .then(|| swift_callable_selector(node, source))
                    .flatten(),
                calls,
                rust_unsafe_calls: BTreeSet::new(),
                direct_authorities,
                rust_regex_match_names: BTreeSet::new(),
                swift_standard_value_names,
            })
        })
        .collect()
}

fn refine_python_stdlib_calls(
    path: &str,
    source: &str,
    sources: &BTreeMap<String, String>,
    functions: &mut [FunctionNode],
) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .is_err()
    {
        return;
    }
    let Some(tree) = parser.parse(source, None) else {
        return;
    };
    let root = tree.root_node();
    let mut shadowed = BTreeSet::new();
    let mut other_bindings = BTreeSet::new();
    for kind in [
        "assignment",
        "augmented_assignment",
        "for_statement",
        "named_expression",
    ] {
        let mut nodes = Vec::new();
        collect_nodes_of_kind(root, kind, &mut nodes);
        for node in nodes {
            if let Some(left) = node
                .child_by_field_name("left")
                .or_else(|| node.child_by_field_name("name"))
            {
                let mut identifiers = Vec::new();
                collect_nodes_of_kind(left, "identifier", &mut identifiers);
                for identifier in identifiers {
                    if let Ok(name) = identifier.utf8_text(source.as_bytes()) {
                        shadowed.insert(name.to_string());
                        if kind != "assignment" {
                            other_bindings.insert(name.to_string());
                        }
                    }
                }
            }
        }
    }
    let mut definitions = Vec::new();
    collect_function_nodes(root, &mut definitions);
    let mut binding_definitions = definitions.clone();
    collect_nodes_of_kind(root, "lambda", &mut binding_definitions);
    collect_nodes_of_kind(root, "class_definition", &mut binding_definitions);
    for definition in &binding_definitions {
        if let Some(name) = function_node_name(*definition, source) {
            shadowed.insert(name);
        }
        if let Some(parameters) = definition.child_by_field_name("parameters") {
            let mut cursor = parameters.walk();
            for parameter in parameters.named_children(&mut cursor) {
                let Some(identifier) = (if parameter.kind() == "identifier" {
                    Some(parameter)
                } else {
                    parameter
                        .child_by_field_name("name")
                        .or_else(|| parameter.named_child(0))
                }) else {
                    continue;
                };
                if let Ok(name) = identifier.utf8_text(source.as_bytes()) {
                    shadowed.insert(name.to_string());
                    other_bindings.insert(name.to_string());
                }
            }
        }
    }
    let mut imports = BTreeMap::<String, Vec<(String, String)>>::new();
    let mut statements = Vec::new();
    collect_nodes_of_kind(root, "import_statement", &mut statements);
    collect_nodes_of_kind(root, "import_from_statement", &mut statements);
    for statement in statements {
        let Ok(line) = statement.utf8_text(source.as_bytes()) else {
            continue;
        };
        let words = line.split_whitespace().collect::<Vec<_>>();
        let (module, member, alias) = match words.as_slice() {
            ["import", module] => (*module, "", *module),
            ["import", module, "as", alias] => (*module, "", *alias),
            ["from", module, "import", member] => (*module, *member, *member),
            ["from", module, "import", member, "as", alias] => (*module, *member, *alias),
            _ => continue,
        };
        if statement.parent() != Some(root) {
            shadowed.insert(alias.to_string());
            continue;
        }
        imports
            .entry(alias.to_string())
            .or_default()
            .push((module.to_string(), member.to_string()));
    }
    let mut recognized = BTreeMap::new();
    for (alias, imported) in imports {
        if imported.len() != 1 || shadowed.contains(&alias) {
            continue;
        }
        let (module, member) = &imported[0];
        if !matches!(module.as_str(), "time" | "shutil" | "pathlib") {
            continue;
        }
        if sources.keys().any(|path| {
            path.ends_with(&format!("/{module}.py"))
                || path == &format!("{module}.py")
                || path.ends_with(&format!("/{module}/__init__.py"))
        }) {
            continue;
        }
        recognized.insert(alias, (module.clone(), member.clone()));
    }
    for function in functions {
        let candidates = definitions
            .iter()
            .filter(|node| {
                function_node_name(**node, source).as_deref() == Some(function.name.as_str())
            })
            .collect::<Vec<_>>();
        if candidates.len() != 1 {
            continue;
        }
        let definition = *candidates[0];
        let mut assignments = Vec::new();
        collect_nodes_of_kind(definition, "assignment", &mut assignments);
        let mut values = BTreeMap::<String, Vec<Node<'_>>>::new();
        for assignment in assignments {
            let (Some(left), Some(right)) = (
                assignment.child_by_field_name("left"),
                assignment.child_by_field_name("right"),
            ) else {
                continue;
            };
            if left.kind() != "identifier" {
                continue;
            }
            if let Ok(name) = left.utf8_text(source.as_bytes()) {
                values.entry(name.to_string()).or_default().push(right);
            }
        }
        let mut paths = BTreeSet::new();
        loop {
            let mut changed = false;
            for (name, values) in &values {
                if values.len() == 1
                    && !other_bindings.contains(name)
                    && python_is_path_value(values[0], source, &recognized, &paths)
                {
                    changed |= paths.insert(name.clone());
                }
            }
            if !changed {
                break;
            }
        }
        function.calls = function
            .calls
            .iter()
            .map(|call| {
                if let Some(receiver) = call.strip_suffix(".stat") {
                    if paths.contains(receiver) {
                        return "python-stdlib.Path.stat".to_string();
                    }
                }
                for (alias, (module, member)) in &recognized {
                    for (operation, target) in [
                        ("sleep", "python-stdlib.time.sleep"),
                        ("copy2", "python-stdlib.shutil.copy2"),
                        ("Path", "PythonStdlibPath"),
                    ] {
                        if !matches!(
                            (module.as_str(), operation),
                            ("time", "sleep") | ("shutil", "copy2") | ("pathlib", "Path")
                        ) {
                            continue;
                        }
                        let expected = if member.is_empty() {
                            format!("{alias}.{operation}")
                        } else {
                            if member != operation {
                                continue;
                            }
                            format!("{}.py#{member}", normalized_import_path(path, module))
                        };
                        if call == &expected {
                            return target.to_string();
                        }
                    }
                }
                call.clone()
            })
            .collect();
        function.direct_authorities = function
            .calls
            .iter()
            .filter_map(|call| authority_for_call("python", call))
            .collect();
    }
}

fn python_is_path_value(
    node: Node<'_>,
    source: &str,
    imports: &BTreeMap<String, (String, String)>,
    paths: &BTreeSet<String>,
) -> bool {
    if node.kind() == "identifier" {
        return node
            .utf8_text(source.as_bytes())
            .is_ok_and(|name| paths.contains(name));
    }
    if node.kind() == "binary_operator"
        && node
            .child_by_field_name("operator")
            .and_then(|operator| operator.utf8_text(source.as_bytes()).ok())
            == Some("/")
    {
        return node
            .child_by_field_name("right")
            .is_some_and(|right| right.kind() == "string")
            && node
                .child_by_field_name("left")
                .is_some_and(|left| python_is_path_value(left, source, imports, paths));
    }
    if node.kind() != "call" {
        return false;
    }
    let Some(callee) = node
        .child_by_field_name("function")
        .and_then(|callee| callee.utf8_text(source.as_bytes()).ok())
    else {
        return false;
    };
    imports.iter().any(|(alias, (module, member))| {
        module == "pathlib"
            && ((member == "Path" && callee == alias)
                || (member.is_empty() && callee == format!("{alias}.Path")))
    })
}

fn python_static_dispatches(
    root: Node<'_>,
    source: &str,
    aliases: &BTreeMap<String, String>,
) -> BTreeMap<String, Vec<String>> {
    let mut assignments = Vec::new();
    collect_nodes_of_kind(root, "assignment", &mut assignments);
    let mut dispatches = BTreeMap::new();
    for assignment in assignments {
        if nearest_function_ancestor(assignment).is_some() {
            continue;
        }
        let Some(name) = assignment
            .child_by_field_name("left")
            .and_then(|left| left.utf8_text(source.as_bytes()).ok())
            .map(str::trim)
            .filter(|name| is_simple_identifier(name))
        else {
            continue;
        };
        let Some(dictionary) = assignment
            .child_by_field_name("right")
            .filter(|right| right.kind() == "dictionary")
        else {
            continue;
        };
        let mut pairs = Vec::new();
        collect_nodes_of_kind(dictionary, "pair", &mut pairs);
        let mut targets = pairs
            .into_iter()
            .filter_map(|pair| pair.child_by_field_name("value"))
            .filter_map(|value| value.utf8_text(source.as_bytes()).ok())
            .map(str::trim)
            .filter(|target| is_simple_identifier(target))
            .map(|target| resolve_call_alias(target, aliases))
            .collect::<Vec<_>>();
        targets.sort();
        targets.dedup();
        if !targets.is_empty() {
            dispatches.insert(name.to_string(), targets);
        }
    }
    dispatches
}

fn is_simple_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn swift_global_standard_names(sources: &BTreeMap<String, String>) -> SwiftStandardNames {
    let language: Language = tree_sitter_swift::LANGUAGE.into();
    let mut subscript_classifications = BTreeMap::<String, bool>::new();
    let mut value_classifications = BTreeMap::<String, bool>::new();
    for source in sources.values() {
        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            continue;
        }
        let Some(tree) = parser.parse(source, None) else {
            continue;
        };
        let mut declarations = Vec::new();
        collect_nodes_of_kind(tree.root_node(), "property_declaration", &mut declarations);
        collect_nodes_of_kind(tree.root_node(), "parameter", &mut declarations);
        for declaration in declarations {
            if nearest_function_ancestor(declaration).is_some() || !has_explicit_type(declaration) {
                continue;
            }
            let Some(name_node) = declaration.child_by_field_name("name") else {
                continue;
            };
            let Some(name) = first_simple_identifier(name_node, source) else {
                continue;
            };
            let is_subscript = has_standard_collection_type(declaration, source);
            let prior_subscript = subscript_classifications
                .get(&name)
                .copied()
                .unwrap_or(true);
            subscript_classifications.insert(name.clone(), prior_subscript && is_subscript);
            let is_value = has_standard_value_type(declaration, source);
            let prior_value = value_classifications.get(&name).copied().unwrap_or(true);
            value_classifications.insert(name, prior_value && is_value);
        }
        for name in swift_labeled_array_value_names(source) {
            subscript_classifications
                .entry(name.clone())
                .or_insert(true);
            value_classifications.entry(name).or_insert(true);
        }
    }
    SwiftStandardNames {
        subscript: subscript_classifications
            .into_iter()
            .filter_map(|(name, standard)| standard.then_some(name))
            .collect(),
        value: value_classifications
            .into_iter()
            .filter_map(|(name, standard)| standard.then_some(name))
            .collect(),
    }
}

fn swift_labeled_array_value_names(source: &str) -> BTreeSet<String> {
    source
        .split(['\n', '(', ')', ','])
        .filter_map(|fragment| {
            let (label, value_type) = fragment.split_once(':')?;
            let name = label.split_whitespace().last()?.trim();
            let value_type = value_type.trim_start();
            (is_simple_identifier(name)
                && (value_type.starts_with('[') || value_type.starts_with("Array<")))
            .then(|| name.to_string())
        })
        .collect()
}

fn tree_sitter_import_aliases(
    binding: &str,
    path: &str,
    root: Node<'_>,
    source: &str,
) -> BTreeMap<String, String> {
    let mut statements = Vec::new();
    collect_nodes_of_kind(
        root,
        if matches!(binding, "js" | "javascript") {
            "import_statement"
        } else {
            "import_from_statement"
        },
        &mut statements,
    );
    let mut aliases = BTreeMap::new();
    for statement in statements {
        let Ok(text) = statement.utf8_text(source.as_bytes()) else {
            continue;
        };
        if matches!(binding, "js" | "javascript") {
            let Some(open) = text.find('{') else { continue };
            let Some(close) = text[open + 1..].find('}').map(|value| open + 1 + value) else {
                continue;
            };
            let Some(from) = text[close + 1..]
                .find("from")
                .map(|value| close + 1 + value)
            else {
                continue;
            };
            let Some(module) = quoted_module(&text[from + 4..]) else {
                continue;
            };
            let resolved = normalized_import_path(path, module);
            for item in text[open + 1..close].split(',') {
                let parts = item.split_whitespace().collect::<Vec<_>>();
                let Some(original) = parts.first().copied().filter(|value| !value.is_empty())
                else {
                    continue;
                };
                let alias = if parts.len() == 3 && parts[1] == "as" {
                    parts[2]
                } else {
                    original
                };
                aliases.insert(alias.to_string(), format!("{resolved}#{original}"));
            }
        } else if binding == "python" {
            let Some((module, imported)) = text
                .strip_prefix("from ")
                .and_then(|text| text.split_once(" import "))
            else {
                continue;
            };
            let pure_stdlib_module = matches!(module, "urllib.parse");
            let resolved = (!pure_stdlib_module)
                .then(|| normalized_import_path(path, &module.replace('.', "/")));
            for item in imported.split(',') {
                let parts = item.split_whitespace().collect::<Vec<_>>();
                let Some(original) = parts.first().copied() else {
                    continue;
                };
                let alias = if parts.len() == 3 && parts[1] == "as" {
                    parts[2]
                } else {
                    original
                };
                let target = resolved
                    .as_ref()
                    .map(|resolved| format!("{resolved}.py#{original}"))
                    .unwrap_or_else(|| format!("{module}.{original}"));
                aliases.insert(alias.to_string(), target);
            }
        }
    }
    aliases
}

fn collect_nodes_of_kind<'tree>(node: Node<'tree>, kind: &str, result: &mut Vec<Node<'tree>>) {
    if node.kind() == kind {
        result.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_nodes_of_kind(child, kind, result);
    }
}

fn quoted_module(source: &str) -> Option<&str> {
    let quote_index = source.find(['\'', '"'])?;
    let quote = source.as_bytes()[quote_index] as char;
    let rest = &source[quote_index + 1..];
    let end = rest.find(quote)?;
    Some(&rest[..end])
}

fn normalized_import_path(current: &str, imported: &str) -> String {
    let mut parts = current
        .replace('\\', "/")
        .split('/')
        .map(str::to_string)
        .collect::<Vec<_>>();
    parts.pop();
    for part in imported.replace('\\', "/").split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            value => parts.push(value.to_string()),
        }
    }
    parts.join("/")
}

fn tree_sitter_language(binding: &str) -> Option<Language> {
    match binding {
        "python" => Some(tree_sitter_python::LANGUAGE.into()),
        "js" | "javascript" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "shell" => Some(tree_sitter_bash::LANGUAGE.into()),
        "swift" => Some(tree_sitter_swift::LANGUAGE.into()),
        _ => None,
    }
}

fn collect_function_nodes<'tree>(node: Node<'tree>, result: &mut Vec<Node<'tree>>) {
    if matches!(
        node.kind(),
        "function_definition" | "function_declaration" | "method_definition" | "init_declaration"
    ) {
        result.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_function_nodes(child, result);
    }
}

fn function_node_name(node: Node<'_>, source: &str) -> Option<String> {
    if node.kind() == "init_declaration" {
        return Some("init".to_string());
    }
    node.child_by_field_name("name")
        .and_then(|name| name.utf8_text(source.as_bytes()).ok())
        .map(|name| name.trim().to_string())
        .or_else(|| {
            let mut cursor = node.walk();
            let result = node
                .named_children(&mut cursor)
                .find(|child| matches!(child.kind(), "identifier" | "simple_identifier"))
                .and_then(|name| name.utf8_text(source.as_bytes()).ok())
                .map(|name| name.trim().to_string());
            result
        })
}

fn tree_sitter_qualified_name(node: Node<'_>, source: &str, name: &str) -> String {
    let mut owners = Vec::new();
    let mut parent = node.parent();
    while let Some(candidate) = parent {
        if matches!(
            candidate.kind(),
            "class_definition"
                | "class_declaration"
                | "struct_declaration"
                | "enum_declaration"
                | "actor_declaration"
        ) {
            if let Some(owner) = candidate
                .child_by_field_name("name")
                .and_then(|name| name.utf8_text(source.as_bytes()).ok())
            {
                owners.push(owner.trim().to_string());
            }
        }
        parent = candidate.parent();
    }
    owners.reverse();
    if owners.is_empty() {
        name.to_string()
    } else {
        format!("{}::{name}", owners.join("::"))
    }
}

fn swift_callable_selector(node: Node<'_>, source: &str) -> Option<String> {
    let declaration = node.utf8_text(source.as_bytes()).ok()?;
    let start = declaration.find('(')?;
    let end = matching_delimiter(declaration, start, '(', ')')?;
    let parameters = &declaration[start + 1..end];
    let mut selectors = Vec::new();
    for parameter in split_top_level(parameters, ',') {
        let parameter = parameter.trim();
        if parameter.is_empty() {
            continue;
        }
        let colon = top_level_delimiter(parameter, ':')?;
        let names = parameter[..colon].split_whitespace().collect::<Vec<_>>();
        let label = *names.first()?;
        let type_end = top_level_delimiter(&parameter[colon + 1..], '=')
            .map(|index| colon + 1 + index)
            .unwrap_or(parameter.len());
        let parameter_type = parameter[colon + 1..type_end]
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        if parameter_type.is_empty() {
            return None;
        }
        selectors.push(format!("{label}:{parameter_type}"));
    }
    Some(format!("({})", selectors.join(",")))
}

fn matching_delimiter(source: &str, start: usize, open: char, close: char) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, character) in source[start..].char_indices() {
        if character == open {
            depth += 1;
        } else if character == close {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(start + offset);
            }
        }
    }
    None
}

fn top_level_delimiter(source: &str, delimiter: char) -> Option<usize> {
    let mut depths = [0usize; 4];
    for (index, character) in source.char_indices() {
        match character {
            '(' => depths[0] += 1,
            ')' => depths[0] = depths[0].saturating_sub(1),
            '[' => depths[1] += 1,
            ']' => depths[1] = depths[1].saturating_sub(1),
            '<' => depths[2] += 1,
            '>' => depths[2] = depths[2].saturating_sub(1),
            '{' => depths[3] += 1,
            '}' => depths[3] = depths[3].saturating_sub(1),
            _ if character == delimiter && depths.iter().all(|depth| *depth == 0) => {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn split_top_level(source: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut remainder = source;
    while let Some(index) = top_level_delimiter(remainder, delimiter) {
        parts.push(&source[start..start + index]);
        start += index + delimiter.len_utf8();
        remainder = &source[start..];
    }
    parts.push(&source[start..]);
    parts
}

fn collect_call_nodes(
    binding: &str,
    node: Node<'_>,
    source: &str,
    swift_collection_names: &BTreeSet<String>,
    calls: &mut BTreeSet<String>,
    root: bool,
) {
    if !root
        && matches!(
            node.kind(),
            "function_definition" | "function_declaration" | "method_definition"
        )
    {
        return;
    }
    if matches!(node.kind(), "call" | "call_expression") {
        let callee = node
            .child_by_field_name("function")
            .or_else(|| node.child_by_field_name("name"))
            .or_else(|| node.named_child(0));
        let call = callee
            .and_then(|callee| callee.utf8_text(source.as_bytes()).ok())
            .map(normalize_call);
        if binding == "swift" && call.as_deref() == Some("defer") {
            // Tree-sitter represents Swift's `defer` control-flow statement as
            // a call. The statement body is still traversed below, so only the
            // keyword itself is excluded from the call graph.
        } else if binding == "swift" && swift_call_is_immediate_closure(callee, source) {
            // The closure body is traversed below. Its calls and authorities
            // remain visible, so invoking this statically present closure does
            // not require dynamic-dispatch authority.
        } else if binding == "swift"
            && swift_call_is_standard_collection_subscript(
                node,
                callee,
                source,
                swift_collection_names,
            )
        {
            // Swift's grammar represents subscripting as a call expression. A
            // subscript on a statically visible standard collection is a value
            // operation, not an unresolved function or dynamic authority.
        } else if binding == "swift"
            && swift_call_is_standard_zip_order_check(node, source, swift_collection_names)
        {
            // This exact standard-library ordering predicate has no open
            // callback. Other higher-order calls remain fail-closed.
        } else if binding == "swift"
            && swift_call_is_standard_literal_collection_operation(
                node,
                source,
                swift_collection_names,
            )
        {
            // Literal closures are traversed below, so their effects remain
            // visible. Only the standard collection dispatch itself is closed.
        } else if binding == "swift" && swift_call_is_boolean_negation(node, callee, source) {
            // Tree-sitter models parenthesized boolean negation as a call with
            // the prefix operator as callee.
        } else if let Some(call) = call {
            calls.insert(call);
        } else {
            calls.insert("<dynamic-call>".to_string());
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_call_nodes(binding, child, source, swift_collection_names, calls, false);
    }
}

fn swift_call_is_boolean_negation(node: Node<'_>, callee: Option<Node<'_>>, source: &str) -> bool {
    callee
        .and_then(|callee| callee.utf8_text(source.as_bytes()).ok())
        .is_some_and(|callee| callee.trim() == "!")
        && node
            .utf8_text(source.as_bytes())
            .ok()
            .is_some_and(|text| text.trim_start().starts_with("!("))
}

fn swift_call_is_standard_literal_collection_operation(
    node: Node<'_>,
    source: &str,
    standard_collection_names: &BTreeSet<String>,
) -> bool {
    let Ok(text) = node.utf8_text(source.as_bytes()) else {
        return false;
    };
    let compact = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if compact.starts_with('[') && compact.contains("].joined(") {
        return true;
    }
    let Some((receiver, arguments)) = compact.split_once(".allSatisfy(") else {
        return false;
    };
    let receiver = receiver.rsplit(['.', ':']).next().unwrap_or(receiver);
    standard_collection_names.contains(receiver)
        && arguments.ends_with(')')
        && (arguments.starts_with('{') || arguments.starts_with("({"))
}

fn swift_call_is_standard_zip_order_check(
    node: Node<'_>,
    source: &str,
    standard_collection_names: &BTreeSet<String>,
) -> bool {
    let Ok(text) = node.utf8_text(source.as_bytes()) else {
        return false;
    };
    let compact = text
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let Some(zip_arguments) = compact
        .strip_prefix("zip(")
        .and_then(|text| text.strip_suffix(").allSatisfy(<)"))
    else {
        return false;
    };
    let Some((first, second)) = zip_arguments.split_once(',') else {
        return false;
    };
    let Some(second_base) = second.strip_suffix(".dropFirst()") else {
        return false;
    };
    first == second_base && standard_collection_names.contains(first)
}

fn collect_shell_call_nodes(
    node: Node<'_>,
    source: &str,
    calls: &mut BTreeSet<String>,
    root: bool,
) {
    if !root && node.kind() == "function_definition" {
        return;
    }
    if node.kind() == "command" {
        let call = node
            .child_by_field_name("name")
            .and_then(|name| name.utf8_text(source.as_bytes()).ok())
            .map(str::trim)
            .filter(|name| {
                !name.is_empty()
                    && name.chars().all(|character| {
                        character.is_ascii_alphanumeric() || "_./-".contains(character)
                    })
            })
            .map(str::to_string)
            .unwrap_or_else(|| "<dynamic-call>".to_string());
        calls.insert(call);
    }
    if node.kind() == "file_redirect" {
        let redirect = node
            .utf8_text(source.as_bytes())
            .unwrap_or_default()
            .replace(' ', "");
        if !redirect.ends_with("/dev/null") {
            calls.insert("shell.file_redirect".to_string());
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_shell_call_nodes(child, source, calls, false);
    }
}

fn swift_call_is_standard_collection_subscript(
    node: Node<'_>,
    callee: Option<Node<'_>>,
    source: &str,
    standard_collection_names: &BTreeSet<String>,
) -> bool {
    let Some(callee) = callee else { return false };
    let Ok(call_text) = node.utf8_text(source.as_bytes()) else {
        return false;
    };
    let Ok(callee_text) = callee.utf8_text(source.as_bytes()) else {
        return false;
    };
    let Some(suffix) = call_text.get(callee_text.len()..) else {
        return false;
    };
    if !suffix.trim_start().starts_with('[') {
        return false;
    }
    let Some(base) = last_simple_identifier(callee, source) else {
        return false;
    };
    standard_collection_names.contains(&base)
}

fn swift_call_is_immediate_closure(callee: Option<Node<'_>>, source: &str) -> bool {
    let Some(callee) = callee else { return false };
    if callee.kind().contains("closure") {
        return true;
    }
    callee
        .utf8_text(source.as_bytes())
        .ok()
        .map(str::trim)
        .is_some_and(|text| text.starts_with('{') && text.ends_with('}'))
}

fn swift_standard_collection_names(
    root: Node<'_>,
    function: Node<'_>,
    source: &str,
) -> BTreeSet<String> {
    let mut classifications = BTreeMap::<String, bool>::new();
    let mut declarations = Vec::new();
    collect_nodes_of_kind(root, "property_declaration", &mut declarations);
    for declaration in declarations {
        let enclosing_function = nearest_function_ancestor(declaration);
        if enclosing_function.is_some_and(|candidate| candidate != function) {
            continue;
        }
        let Some(name_node) = declaration.child_by_field_name("name") else {
            continue;
        };
        let Some(name) = first_simple_identifier(name_node, source) else {
            continue;
        };
        if has_explicit_type(declaration) {
            let is_standard = has_standard_collection_type(declaration, source);
            let prior = classifications.get(&name).copied().unwrap_or(true);
            classifications.insert(name, prior && is_standard);
        }
    }

    let mut parameters = Vec::new();
    collect_nodes_of_kind(function, "parameter", &mut parameters);
    for parameter in parameters {
        let Some(name_node) = parameter.child_by_field_name("name") else {
            continue;
        };
        let Some(name) = first_simple_identifier(name_node, source) else {
            continue;
        };
        if has_standard_collection_type(parameter, source) {
            classifications.insert(name, true);
        }
    }

    let mut standard = classifications
        .iter()
        .filter(|(_, is_standard)| **is_standard)
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();

    let mut aliases = Vec::new();
    let mut local_declarations = Vec::new();
    collect_nodes_of_kind(function, "property_declaration", &mut local_declarations);
    for declaration in local_declarations {
        if has_explicit_type(declaration) {
            continue;
        }
        let Some(name_node) = declaration.child_by_field_name("name") else {
            continue;
        };
        let Some(name) = first_simple_identifier(name_node, source) else {
            continue;
        };
        let Some(value) = declaration.child_by_field_name("value") else {
            continue;
        };
        let value_text = value
            .utf8_text(source.as_bytes())
            .unwrap_or_default()
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        if value_text.starts_with('[') || value_text.starts_with("Array(") {
            standard.insert(name.clone());
            continue;
        }
        if let Some(receiver) = value_text.strip_suffix(".sorted()") {
            let receiver = receiver.rsplit(['.', ':']).next().unwrap_or(receiver);
            if standard.contains(receiver) {
                standard.insert(name.clone());
                continue;
            }
        }
        if let Some(source_name) = last_simple_identifier(value, source) {
            aliases.push((name, source_name));
        }
    }
    loop {
        let mut changed = false;
        for (alias, source_name) in &aliases {
            if standard.contains(source_name) {
                changed |= standard.insert(alias.clone());
            }
        }
        if !changed {
            break;
        }
    }
    standard
}

fn swift_standard_value_names(
    root: Node<'_>,
    function: Node<'_>,
    source: &str,
    collection_names: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut standard = collection_names.clone();
    let mut declarations = Vec::new();
    collect_nodes_of_kind(root, "property_declaration", &mut declarations);
    for declaration in declarations {
        let enclosing_function = nearest_function_ancestor(declaration);
        if enclosing_function.is_some_and(|candidate| candidate != function) {
            continue;
        }
        let Some(name_node) = declaration.child_by_field_name("name") else {
            continue;
        };
        let Some(name) = first_simple_identifier(name_node, source) else {
            continue;
        };
        if has_standard_value_type(declaration, source) {
            standard.insert(name);
        }
    }
    let mut parameters = Vec::new();
    collect_nodes_of_kind(function, "parameter", &mut parameters);
    for parameter in parameters {
        let Some(name_node) = parameter.child_by_field_name("name") else {
            continue;
        };
        let Some(name) = first_simple_identifier(name_node, source) else {
            continue;
        };
        if has_standard_value_type(parameter, source) {
            standard.insert(name);
        }
    }
    standard
}

fn nearest_function_ancestor(node: Node<'_>) -> Option<Node<'_>> {
    let mut parent = node.parent();
    while let Some(candidate) = parent {
        if matches!(
            candidate.kind(),
            "function_definition" | "function_declaration" | "method_definition"
        ) {
            return Some(candidate);
        }
        parent = candidate.parent();
    }
    None
}

fn has_explicit_type(node: Node<'_>) -> bool {
    node.child_by_field_name("type").is_some()
        || node.child_by_field_name("type_annotation").is_some()
        || node_has_kind(node, "type_annotation")
}

fn has_standard_collection_type(node: Node<'_>, source: &str) -> bool {
    if node_has_kind(node, "array_type") || node_has_kind(node, "dictionary_type") {
        return true;
    }
    let Ok(text) = node.utf8_text(source.as_bytes()) else {
        return false;
    };
    text.contains("Array<") || text.contains("Dictionary<") || text.contains("Set<")
}

fn has_standard_value_type(node: Node<'_>, source: &str) -> bool {
    if has_standard_collection_type(node, source) || node_has_kind(node, "optional_type") {
        return true;
    }
    let Ok(text) = node.utf8_text(source.as_bytes()) else {
        return false;
    };
    text.contains("Set<")
        || text.contains("Optional<")
        || [
            "Bool", "Int", "Int8", "Int16", "Int32", "Int64", "String", "UInt", "UInt8", "UInt16",
            "UInt32", "UInt64",
        ]
        .iter()
        .any(|name| {
            text.split(|character: char| !character.is_ascii_alphanumeric())
                .any(|part| part == *name)
        })
}

fn node_has_kind(node: Node<'_>, kind: &str) -> bool {
    if node.kind() == kind {
        return true;
    }
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .any(|child| node_has_kind(child, kind));
    found
}

fn first_simple_identifier(node: Node<'_>, source: &str) -> Option<String> {
    if matches!(node.kind(), "identifier" | "simple_identifier") {
        return node
            .utf8_text(source.as_bytes())
            .ok()
            .map(str::trim)
            .map(str::to_string);
    }
    let mut cursor = node.walk();
    let identifier = node
        .named_children(&mut cursor)
        .find_map(|child| first_simple_identifier(child, source));
    identifier
}

fn last_simple_identifier(node: Node<'_>, source: &str) -> Option<String> {
    if matches!(node.kind(), "identifier" | "simple_identifier") {
        return node
            .utf8_text(source.as_bytes())
            .ok()
            .map(str::trim)
            .map(str::to_string);
    }
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter_map(|child| last_simple_identifier(child, source))
        .last()
}

fn normalize_call(call: &str) -> String {
    call.split_whitespace()
        .collect::<String>()
        .trim_start_matches('!')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expectation(
        symbol: &str,
        purity: &str,
        authorities: &[&str],
    ) -> SemanticFunctionExpectation {
        SemanticFunctionExpectation {
            id: "subject".to_string(),
            symbol: symbol.to_string(),
            purity: purity.to_string(),
            authorities: authorities
                .iter()
                .map(|value| (*value).to_string())
                .collect(),
        }
    }

    fn report(
        binding: &str,
        path: &str,
        source: &str,
        expectation: SemanticFunctionExpectation,
    ) -> EffectAnalysis {
        analyze(AnalysisInput {
            binding: binding.to_string(),
            source_digest: "source".to_string(),
            tool_digest: "tool".to_string(),
            sources: BTreeMap::from([(path.to_string(), source.to_string())]),
            semantic_functions: vec![expectation],
            authority_facades: Vec::new(),
            trusted_external_calls: BTreeSet::new(),
        })
    }

    #[test]
    fn rust_transitive_filesystem_effect_is_inferred() {
        let result = report(
            "rust",
            "src/lib.rs",
            "fn decide() { helper(); } fn helper() { std::fs::read_to_string(\"x\").ok(); }",
            expectation("src/lib.rs#decide", "effectful", &["filesystem"]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert_eq!(result.functions[0].resolved_callees, vec!["helper"]);
    }

    #[test]
    fn pure_rust_function_rejects_transitive_effect() {
        let result = report(
            "rust",
            "src/lib.rs",
            "fn decide() { helper(); } fn helper() { std::fs::read_to_string(\"x\").ok(); }",
            expectation("src/lib.rs#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Fail);
        assert!(result.functions[0].reasons[0].contains("filesystem"));
    }

    #[test]
    fn rust_module_qualified_associated_calls_resolve_exactly() {
        let result = analyze(AnalysisInput {
            binding: "rust".to_string(),
            source_digest: "source".to_string(),
            tool_digest: "tool".to_string(),
            sources: BTreeMap::from([
                (
                    "src/transition.rs".to_string(),
                    r#"
                        use crate::representation::{Choice, Decision};
                        fn select() { Choice::from_parts(); Decision::from_parts(); }
                    "#
                    .to_string(),
                ),
                (
                    "src/representation.rs".to_string(),
                    r#"
                        struct Choice;
                        impl Choice { fn from_parts() {} }
                        struct Decision;
                        impl Decision { fn from_parts() {} }
                    "#
                    .to_string(),
                ),
            ]),
            semantic_functions: vec![expectation("src/transition.rs#select", "pure", &[])],
            authority_facades: Vec::new(),
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert_eq!(
            result.functions[0].resolved_callees,
            vec!["Choice::from_parts", "Decision::from_parts"]
        );
        assert!(result.functions[0].unresolved_calls.is_empty());
    }

    #[test]
    fn rust_module_qualified_associated_calls_do_not_cross_modules() {
        let result = analyze(AnalysisInput {
            binding: "rust".to_string(),
            source_digest: "source".to_string(),
            tool_digest: "tool".to_string(),
            sources: BTreeMap::from([
                (
                    "src/transition.rs".to_string(),
                    "fn select() { crate::missing::Choice::from_parts(); }".to_string(),
                ),
                (
                    "src/representation.rs".to_string(),
                    "struct Choice; impl Choice { fn from_parts() {} }".to_string(),
                ),
                (
                    "src/other.rs".to_string(),
                    "struct Choice; impl Choice { fn from_parts() {} }".to_string(),
                ),
            ]),
            semantic_functions: vec![expectation("src/transition.rs#select", "pure", &[])],
            authority_facades: Vec::new(),
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
        assert_eq!(
            result.functions[0].unresolved_calls,
            vec!["crate::missing::Choice::from_parts"]
        );
    }

    #[test]
    fn rust_ordering_combinators_are_pure_while_their_comparators_are_traversed() {
        let pure = report(
            "rust",
            "src/transition.rs",
            r#"
                fn select(values: &[u8]) {
                    values.iter().min_by(|left, right| left.cmp(right));
                    values.binary_search_by(|candidate| candidate.cmp(&0));
                }
            "#,
            expectation("select", "pure", &[]),
        );
        assert_eq!(pure.result, AnalysisResult::Pass, "{pure:#?}");

        let effectful_comparator = report(
            "rust",
            "src/transition.rs",
            r#"
                fn select(values: &[u8]) {
                    values.iter().min_by(|left, right| {
                        std::fs::read_to_string("comparison-order").ok();
                        left.cmp(right)
                    });
                    values.binary_search_by(|candidate| {
                        std::fs::read_to_string("binary-search-order").ok();
                        candidate.cmp(&0)
                    });
                }
            "#,
            expectation("select", "pure", &[]),
        );
        assert_eq!(effectful_comparator.result, AnalysisResult::Fail);
        assert_eq!(
            effectful_comparator.functions[0].transitive_authorities,
            vec!["filesystem"]
        );
    }

    #[test]
    fn rust_iterator_chain_does_not_reach_an_unrelated_same_leaf_method() {
        let result = analyze(AnalysisInput {
            binding: "rust".to_string(),
            source_digest: "source".to_string(),
            tool_digest: "tool".to_string(),
            sources: BTreeMap::from([
                (
                    "src/transition.rs".to_string(),
                    "fn decide(values: &[u64]) -> Option<u64> { values.iter().copied().next() }"
                        .to_string(),
                ),
                (
                    "src/property_support.rs".to_string(),
                    "struct SplitMix64(u64); impl SplitMix64 { fn next(&mut self) -> u64 { self.0.wrapping_mul(3) } }"
                        .to_string(),
                ),
            ]),
            semantic_functions: vec![expectation("src/transition.rs#decide", "pure", &[])],
            authority_facades: Vec::new(),
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(!result.functions[0]
            .resolved_callees
            .contains(&"SplitMix64::next".to_string()));
    }

    #[test]
    fn rust_primitive_wrapping_multiplication_is_pure() {
        let result = report(
            "rust",
            "src/property_support.rs",
            "fn advance(value: u64) -> u64 { value.wrapping_mul(3) }",
            expectation("advance", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn rust_domain_command_associated_calls_are_not_process_authority() {
        let result = report(
            "rust",
            "src/transition.rs",
            "enum TalkTurnCommand { Request } impl TalkTurnCommand { fn kind(&self) {} } fn decide(command: &TalkTurnCommand) { command.kind(); }",
            expectation("decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.is_empty());
    }

    #[test]
    fn rust_typed_local_methods_resolve_exact_receiver() {
        let result = report(
            "rust",
            "src/transition.rs",
            r#"
                struct Pure;
                impl Pure { fn evaluate_candidate(&self) {} }
                struct Effectful;
                impl Effectful {
                    fn evaluate_candidate(&self) { std::fs::read_to_string("evidence").ok(); }
                }
                fn select(value: &Pure) { value.evaluate_candidate(); }
            "#,
            expectation("select", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert_eq!(
            result.functions[0].resolved_callees,
            vec!["Pure::evaluate_candidate"]
        );
    }

    #[test]
    fn rust_fixed_callback_specializations_do_not_merge_callers_or_shadowed_bindings() {
        let definitions = "fn invoke<F: FnOnce()>(callback: F) { callback(); } fn pure_callback() {} fn effect_callback() { std::fs::read_to_string(\"x\").ok(); } #[cfg(test)] mod tests { fn test_only() { super::invoke(super::effect_callback); } }";
        for (body, expected) in [
            ("invoke(pure_callback);", AnalysisResult::Pass),
            ("invoke(effect_callback);", AnalysisResult::Fail),
            (
                "invoke(pure_callback); invoke(effect_callback);",
                AnalysisResult::Fail,
            ),
            (
                "let pure_callback = effect_callback; invoke(pure_callback);",
                AnalysisResult::Fail,
            ),
            (
                "let invoke = unknown(); invoke(pure_callback);",
                AnalysisResult::Fail,
            ),
        ] {
            let result = report(
                "rust",
                "src/lib.rs",
                &format!("{definitions} fn decide() {{ {body} }}"),
                expectation("decide", "pure", &[]),
            );
            assert_eq!(result.result, expected, "{body}: {result:#?}");
        }
        let result = report(
            "rust",
            "src/lib.rs",
            definitions,
            expectation("invoke", "pure", &[]),
        );
        assert_eq!(
            result.result,
            AnalysisResult::Fail,
            "open helper must remain dynamic: {result:#?}"
        );
        for helper in [
            "fn invoke(mut callback: fn()) { callback = effect_callback; callback(); }",
            "fn invoke(mut callback: fn()) { std::mem::replace(&mut callback, effect_callback); callback(); }",
            "fn invoke(callback: fn()) { let callback = effect_callback; callback(); }",
        ] {
            let source = format!("{helper} fn pure_callback() {{}} fn effect_callback() {{ std::fs::read_to_string(\"x\").ok(); }} fn decide() {{ invoke(pure_callback); }}");
            assert_eq!(report("rust", "src/lib.rs", &source, expectation("decide", "pure", &[])).result, AnalysisResult::Fail, "{helper}");
        }
    }

    #[test]
    fn rust_callback_import_uses_verified_crate_identity_and_reexport() {
        let mut sources = BTreeMap::from([
            ("src/lib.rs".into(), "use decisions::transition as decide_delivery; fn transition() {} fn invoke<F: FnOnce()>(callback: F) { callback(); } fn decide() { invoke(decide_delivery); }".into()),
            ("dependencies/decisions/src/lib.rs".into(), "pub use crate::transition::transition;".into()),
            ("dependencies/decisions/src/transition.rs".into(), "pub fn transition() { std::fs::read_to_string(\"x\").ok(); }".into()),
            ("rms-metadata/rust-crate-alias/decisions".into(), "dependencies/decisions".into()),
        ]);
        let run = |sources: BTreeMap<String, String>| {
            analyze(AnalysisInput {
                binding: "rust".into(),
                source_digest: "source".into(),
                tool_digest: "tool".into(),
                sources,
                semantic_functions: vec![expectation("src/lib.rs#decide", "pure", &[])],
                authority_facades: Vec::new(),
                trusted_external_calls: BTreeSet::new(),
            })
        };
        let effectful = run(sources.clone());
        assert_eq!(effectful.result, AnalysisResult::Fail, "{effectful:#?}");
        assert!(effectful.functions[0]
            .transitive_authorities
            .contains(&"filesystem".into()));
        sources.insert(
            "dependencies/decisions/src/transition.rs".into(),
            "pub fn transition() {}".into(),
        );
        assert_eq!(run(sources.clone()).result, AnalysisResult::Pass);
        sources.remove("rms-metadata/rust-crate-alias/decisions");
        assert_eq!(run(sources).result, AnalysisResult::Fail);
    }

    #[test]
    fn rust_direct_reexports_are_exact_and_ambiguous_globs_fail_closed() {
        let mut sources = BTreeMap::from([
            (
                "src/lib.rs".into(),
                "pub use crate::one::*; pub use crate::two::*; fn decide() { crate::run(); }"
                    .into(),
            ),
            ("src/one.rs".into(), "pub fn run() {}".into()),
            ("src/two.rs".into(), "pub fn other() {}".into()),
        ]);
        let run = |sources: BTreeMap<String, String>| {
            analyze(AnalysisInput {
                binding: "rust".into(),
                source_digest: "source".into(),
                tool_digest: "tool".into(),
                sources,
                semantic_functions: vec![expectation("src/lib.rs#decide", "pure", &[])],
                authority_facades: Vec::new(),
                trusted_external_calls: BTreeSet::new(),
            })
        };
        assert_eq!(run(sources.clone()).result, AnalysisResult::Pass);
        sources.insert(
            "src/two.rs".into(),
            "pub fn run() { std::fs::read_to_string(\"x\").ok(); }".into(),
        );
        assert_eq!(run(sources).result, AnalysisResult::Fail);
        let unknown = report(
            "rust",
            "src/lib.rs",
            "fn run() {} fn decide() { unverified_dependency::run(); }",
            expectation("decide", "pure", &[]),
        );
        assert_eq!(unknown.result, AnalysisResult::Fail);
    }

    #[test]
    fn rust_declared_return_and_iterator_receiver_types_are_bounded() {
        let definitions = "struct Root; struct Leaf; impl Root { fn items(&self) -> &[Leaf] { todo!() } fn leaf(&self) -> &Leaf { todo!() } } impl Leaf { fn evaluate(&self) {} fn mutate(&self) { std::fs::read_to_string(\"x\").ok(); } }";
        for (body, expected) in [
            ("root.leaf().evaluate();", AnalysisResult::Pass),
            ("root.items().iter().map(|item| item.evaluate()).count();", AnalysisResult::Pass),
            ("root.leaf().mutate();", AnalysisResult::Fail),
            ("root.items().iter().map(|item| item.mutate()).count();", AnalysisResult::Fail),
            ("root.items().iter().map(|item| { let item = unknown(); item.evaluate(); }).count();", AnalysisResult::Fail),
            ("[()].iter().map(|root| root.evaluate()).count();", AnalysisResult::Fail),
        ] {
            let result = report("rust", "src/lib.rs", &format!("{definitions} fn decide(root: &Root) {{ {body} }}"), expectation("decide", "pure", &[]));
            assert_eq!(result.result, expected, "{body}: {result:#?}");
        }
    }

    #[test]
    fn rust_parameter_type_does_not_leak_to_fields_returns_or_shadowed_closures() {
        for body in [
            "value.score.loss_permille();",
            "value.next().loss_permille();",
            "[()].iter().map(|value| value.loss_permille()).count();",
        ] {
            let source = format!(
                "struct Root; impl Root {{ fn loss_permille(&self) {{}} fn next(&self) {{}} }} fn select(value: &Root) {{ {body} }}"
            );
            let result = report(
                "rust",
                "src/lib.rs",
                &source,
                expectation("select", "pure", &[]),
            );
            assert_eq!(result.result, AnalysisResult::Fail, "{body}: {result:#?}");
            assert!(
                !result.functions[0]
                    .resolved_callees
                    .contains(&"Root::loss_permille".to_string()),
                "{body}: {result:#?}"
            );
        }
    }

    #[test]
    fn swift_unqualified_calls_stay_in_their_source_module() {
        let mut nodes = Vec::new();
        for (path, source) in [
            (
                "Sources/Adapter.swift",
                "func adapter() { transitionRecord() }",
            ),
            ("Sources/Transition.swift", "func transitionRecord() {}"),
            (
                "dependencies/provider/Sources/Transition.swift",
                "func transitionRecord() {}",
            ),
        ] {
            nodes.extend(extract_tree_sitter_functions(
                "swift",
                path,
                source,
                &SwiftStandardNames::default(),
            ));
        }
        let resolved = resolve_local_call(0, "transitionRecord", &nodes);
        assert_eq!(resolved, vec![1]);
        let resolved = resolve_local_call(2, "transitionRecord", &nodes);
        assert_eq!(resolved, vec![2]);
        nodes.push(nodes[1].clone());
        assert_eq!(resolve_local_call(0, "transitionRecord", &nodes).len(), 2);
    }

    #[test]
    fn rust_local_definitions_precede_external_call_heuristics() {
        let local_transport = report(
            "rust",
            "src/representation.rs",
            "struct Observation; impl Observation { fn tcp(&self) {} } fn select(value: &Observation) { value.tcp(); }",
            expectation("select", "pure", &[]),
        );
        assert_eq!(
            local_transport.result,
            AnalysisResult::Pass,
            "{local_transport:#?}"
        );

        let local_known_name = report(
            "rust",
            "src/adapter.rs",
            r#"
                struct Adapter;
                impl Adapter {
                    fn inspect(&self) { std::fs::read_to_string("evidence").ok(); }
                }
                fn select(value: &Adapter) { value.inspect(); }
            "#,
            expectation("select", "pure", &[]),
        );
        assert_eq!(
            local_known_name.result,
            AnalysisResult::Fail,
            "{local_known_name:#?}"
        );
        assert_eq!(
            local_known_name.functions[0].transitive_authorities,
            vec!["filesystem"]
        );
    }

    #[test]
    fn rust_static_metadata_and_standard_value_queries_stay_pure() {
        let source = r#"
            struct Metadata { region: &'static str }
            static METADATA: &[Metadata] = &[];
            fn region_metadata(region: &str) -> Option<&'static Metadata> {
                METADATA.iter().find(|item| item.region == region)
            }
            struct E164(String);
            impl E164 {
                fn new(value: String) -> Option<Self> {
                    let digits = value.strip_prefix('+')?;
                    if digits.bytes().all(|byte| byte.is_ascii_digit()) {
                        Some(Self(value))
                    } else {
                        None
                    }
                }
            }
            fn owned(value: &str) -> String { value.to_owned() }
        "#;
        for symbol in ["region_metadata", "E164::new", "owned"] {
            let result = report(
                "rust",
                "src/representation.rs",
                source,
                expectation(symbol, "pure", &[]),
            );
            assert_eq!(result.result, AnalysisResult::Pass, "{symbol}: {result:#?}");
        }
        let filesystem = report(
            "rust",
            "src/adapter.rs",
            "fn inspect(path: &std::path::Path) { std::fs::metadata(path).ok(); }",
            expectation("inspect", "effectful", &["filesystem"]),
        );
        assert_eq!(filesystem.result, AnalysisResult::Pass, "{filesystem:#?}");
    }

    #[test]
    fn rust_owned_map_values_are_a_pure_collection_iterator() {
        let result = report(
            "rust",
            "src/transition.rs",
            "fn select(values: std::collections::BTreeMap<u64, u64>) -> Vec<u64> { values.into_values().collect() }",
            expectation("select", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.is_empty());
    }

    #[test]
    fn rust_verified_dependency_sources_close_named_projector_calls() {
        let result = analyze(AnalysisInput {
            binding: "rust".to_string(),
            source_digest: "source".to_string(),
            tool_digest: "tool".to_string(),
            sources: BTreeMap::from([
                ("src/consumer.rs".to_string(), "fn project(value: &Provider) -> u16 { value.port() } fn decide(values: &[Provider]) -> Vec<u16> { values.iter().map(project).collect() }".to_string()),
                ("dependencies/provider/src/lib.rs".to_string(), "struct Provider { port: u16 } impl Provider { fn port(&self) -> u16 { self.port } }".to_string()),
            ]),
            semantic_functions: vec![expectation("src/consumer.rs#decide", "pure", &[])],
            authority_facades: vec![],
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0]
            .resolved_callees
            .contains(&"project".to_string()));
        assert!(result.functions[0]
            .resolved_callees
            .contains(&"Provider::port".to_string()));
    }

    #[test]
    fn rust_dynamic_iterator_callback_remains_fail_closed() {
        let source = "fn decide<F: Fn(&u64) -> u64>(values: &[u64], project: F) -> Vec<u64> { values.iter().map(project).collect() }";
        let result = report(
            "rust",
            "src/lib.rs",
            source,
            expectation("decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
        assert_eq!(
            result.functions[0].transitive_authorities,
            vec!["dynamic-dispatch"]
        );
    }

    #[test]
    fn rust_primitive_checked_conversion_is_pure() {
        let result = report(
            "rust",
            "src/lib.rs",
            "fn decide(value: u64) -> Option<u16> { u16::try_from(value).ok() }",
            expectation("decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn rust_provider_domain_vocabulary_does_not_imply_external_authority() {
        let result = report(
            "rust",
            "src/parser.rs",
            "struct StunProviderCandidates; impl StunProviderCandidates { fn new() -> Self { Self } } fn parse() -> StunProviderCandidates { StunProviderCandidates::new() }",
            expectation("parse", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.is_empty());
    }

    #[test]
    fn rust_native_memory_primitives_and_declared_pure_dependency_are_classified_exactly() {
        let result = analyze(AnalysisInput {
            binding: "rust".to_string(),
            source_digest: "source".to_string(),
            tool_digest: "tool".to_string(),
            sources: BTreeMap::from([
                (
                    "src/adapter.rs".to_string(),
                    "fn normalize(bytes: &mut [u8]) { bytes.copy_from_slice(&[1]); phone_number_normalization::normalize_phone_number(); }".to_string(),
                ),
                (
                    "src/ffi.rs".to_string(),
                    "pub unsafe extern \"C\" fn invoke(output: *mut u8) { let _ = std::mem::align_of::<u8>(); let _ = std::mem::size_of::<u8>(); let _ = std::panic::catch_unwind(|| unsafe { let _ = std::slice::from_raw_parts(output, 1); output.write(1); }); }".to_string(),
                ),
            ]),
            semantic_functions: vec![
                expectation("src/adapter.rs#normalize", "pure", &[]),
                SemanticFunctionExpectation {
                    id: "ffi".to_string(),
                    symbol: "src/ffi.rs#invoke".to_string(),
                    purity: "effectful".to_string(),
                    authorities: BTreeSet::from(["foreign-memory".to_string()]),
                },
            ],
            authority_facades: vec![AuthorityFacade {
                authority: "foreign-memory".to_string(),
                symbol: "src/ffi.rs#invoke".to_string(),
            }],
            trusted_external_calls: BTreeSet::from([
                "phone_number_normalization::normalize_phone_number".to_string(),
            ]),
        });

        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result
            .functions
            .iter()
            .all(|function| function.unresolved_calls.is_empty()));
        assert_eq!(
            result
                .functions
                .iter()
                .find(|function| function.id == "ffi")
                .unwrap()
                .transitive_authorities,
            vec!["foreign-memory"]
        );

        let unknown = report(
            "rust",
            "src/adapter.rs",
            "fn normalize() { unbound_crate::normalize_phone_number(); }",
            expectation("normalize", "pure", &[]),
        );
        assert_eq!(unknown.result, AnalysisResult::Fail, "{unknown:#?}");
    }

    #[test]
    fn rust_regex_read_queries_stay_pure_without_hiding_dynamic_callbacks() {
        let regex = report(
            "rust",
            "src/normalization.rs",
            "fn format_number(regex: &Regex, number: &str) -> String { let _ = '1'.to_digit(10); let mut owned = number.to_owned(); if regex.is_match(number) { if let Some(captures) = regex.captures(number) { if let Some(matched) = captures.get(0) { owned.replace_range(..matched.end(), number); } } } owned }",
            expectation("format_number", "pure", &[]),
        );
        assert_eq!(regex.result, AnalysisResult::Pass, "{regex:#?}");

        let match_offset = report(
            "rust",
            "src/normalization.rs",
            "fn has_extension(raw: &str) -> bool { let Ok(suffix) = Regex::new(\"ext$\") else { return false; }; let Some(found) = suffix.find(raw) else { return false; }; found.start() > 0 }",
            expectation("has_extension", "pure", &[]),
        );
        assert_eq!(
            match_offset.result,
            AnalysisResult::Pass,
            "{match_offset:#?}"
        );

        let unknown_start = report(
            "rust",
            "src/runtime.rs",
            "fn launch(service: &Service) { service.start(); }",
            expectation("launch", "pure", &[]),
        );
        assert_eq!(
            unknown_start.result,
            AnalysisResult::Fail,
            "{unknown_start:#?}"
        );
        assert!(unknown_start.functions[0]
            .transitive_authorities
            .contains(&"dynamic-dispatch".to_string()));

        let dynamic = report(
            "rust",
            "src/normalization.rs",
            "fn decide(callback: impl Fn(&str), number: &str) { callback(number); }",
            expectation("decide", "pure", &[]),
        );
        assert_eq!(dynamic.result, AnalysisResult::Fail, "{dynamic:#?}");
        assert!(dynamic.functions[0]
            .transitive_authorities
            .contains(&"dynamic-dispatch".to_string()));
    }

    #[test]
    fn comments_and_strings_do_not_create_javascript_calls() {
        let result = report(
            "js",
            "src/index.js",
            "function decide(value) { /* fetch(value) */ return 'fetch(value)' + value; }",
            expectation("src/index.js#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn python_dynamic_calls_fail_closed() {
        let result = report(
            "python",
            "src/app.py",
            "def decide(callback, value):\n    return callback(value)\n",
            expectation("src/app.py#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Fail);
        assert_eq!(result.functions[0].unresolved_calls, vec!["callback"]);
    }

    #[test]
    fn python_urllib_parse_import_resolves_as_pure_stdlib() {
        let source = "from urllib.parse import urlsplit\ndef decide(value):\n    return urlsplit(value).scheme\n";
        let result = report(
            "python",
            "src/package/transition.py",
            source,
            expectation("decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.is_empty());
        assert!(result.functions[0].unresolved_calls.is_empty());
    }

    #[test]
    fn python_stdlib_effects_resolve_exact_imports_and_path_division() {
        for (source, authority) in [
            ("import time\ndef run():\n    time.sleep(1)\n", "clock"),
            ("import shutil\ndef run():\n    shutil.copy2('a', 'b')\n", "filesystem"),
            ("from pathlib import Path\ndef run():\n    root = Path('dir')\n    child = root / 'file'\n    return child.stat()\n", "filesystem"),
        ] {
            let result = report("python", "scripts/main.py", source, expectation("run", "effectful", &[authority]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        }
    }

    #[test]
    fn python_stdlib_refinement_preserves_unknown_and_shadowed_receivers() {
        for source in [
            "import time\ndef run(time):\n    time.sleep(1)\n",
            "import time\ndef run():\n    import custom as time\n    time.sleep(1)\n",
            "import shutil\ndef run(shutil):\n    shutil.copy2('a', 'b')\n",
            "from pathlib import Path\ndef run(Path):\n    root = Path('dir')\n    return root.stat()\n",
            "def run(root):\n    return root.stat()\n",
            "from pathlib import Path\ndef run(other):\n    root = Path('dir')\n    root = other\n    return root.stat()\n",
            "from pathlib import Path\ndef run():\n    root = Path('dir')\n    return lambda root: root.stat()\n",
            "from pathlib import Path\ndef run(other):\n    root = Path('dir') / other\n    return root.stat()\n",
        ] {
            let result = report("python", "scripts/main.py", source, expectation("run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
            assert!(result.functions[0].transitive_authorities.contains(&"dynamic-dispatch".to_string()), "{result:#?}");
        }
        let mut nodes = extract_tree_sitter_functions(
            "python",
            "scripts/main.py",
            "import time\ndef run():\n    time.sleep(1)\n",
            &SwiftStandardNames::default(),
        );
        let sources = BTreeMap::from([("scripts/time.py".to_string(), "".to_string())]);
        refine_python_stdlib_calls(
            "scripts/main.py",
            "import time\ndef run():\n    time.sleep(1)\n",
            &sources,
            &mut nodes,
        );
        assert!(!nodes[0].calls.contains("python-stdlib.time.sleep"));
    }

    #[test]
    fn shell_local_call_closure_stays_pure() {
        let result = report(
            "shell",
            "scripts/domain.sh",
            "helper() { printf '%s\\n' \"$1\"; }\ndecide() { helper \"$1\"; }\n",
            expectation("scripts/domain.sh#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert_eq!(result.functions[0].resolved_callees, vec!["helper"]);
    }

    #[test]
    fn shell_unknown_and_dynamic_commands_fail_closed() {
        for source in [
            "decide() { unknown_tool \"$1\"; }\n",
            "decide() { \"$CALLBACK\" \"$1\"; }\n",
        ] {
            let result = report(
                "shell",
                "scripts/domain.sh",
                source,
                expectation("scripts/domain.sh#decide", "pure", &[]),
            );
            assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
            assert!(!result.functions[0].unresolved_calls.is_empty());
        }
    }

    #[test]
    fn shell_file_redirection_is_an_effect_but_dev_null_is_not() {
        let effectful = report(
            "shell",
            "scripts/domain.sh",
            "decide() { printf '%s\\n' accepted > result.txt; }\n",
            expectation("scripts/domain.sh#decide", "pure", &[]),
        );
        assert_eq!(effectful.result, AnalysisResult::Fail, "{effectful:#?}");
        assert_eq!(
            effectful.functions[0].transitive_authorities,
            vec!["filesystem"]
        );

        let discarded = report(
            "shell",
            "scripts/domain.sh",
            "decide() { printf '%s\\n' accepted 2>/dev/null; }\n",
            expectation("scripts/domain.sh#decide", "pure", &[]),
        );
        assert_eq!(discarded.result, AnalysisResult::Pass, "{discarded:#?}");
    }

    #[test]
    fn shell_local_function_shadows_external_command_name() {
        let result = report(
            "shell",
            "scripts/domain.sh",
            "git() { printf '%s\\n' local; }\ndecide() { git; }\n",
            expectation("scripts/domain.sh#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert_eq!(result.functions[0].resolved_callees, vec!["git"]);
    }

    #[test]
    fn executable_binding_selects_analyzer_from_each_symbol_path() {
        let result = analyze(AnalysisInput {
            binding: "executable".to_string(),
            source_digest: "source".to_string(),
            tool_digest: "tool".to_string(),
            sources: BTreeMap::from([
                (
                    "scripts/domain.sh".to_string(),
                    "decide() { printf '%s\\n' accepted; }\n".to_string(),
                ),
                (
                    "scripts/parser.py".to_string(),
                    "def parse(value):\n    return value.strip()\n".to_string(),
                ),
            ]),
            semantic_functions: vec![
                expectation("scripts/domain.sh#decide", "pure", &[]),
                SemanticFunctionExpectation {
                    id: "parser".to_string(),
                    symbol: "scripts/parser.py#parse".to_string(),
                    purity: "pure".to_string(),
                    authorities: BTreeSet::new(),
                },
            ],
            authority_facades: Vec::new(),
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert_eq!(result.functions.len(), 2);
    }

    #[test]
    fn static_python_dispatch_preserves_exact_authority_facade() {
        let result = analyze(AnalysisInput {
            binding: "executable".to_string(),
            source_digest: "source".to_string(),
            tool_digest: "tool".to_string(),
            sources: BTreeMap::from([
                (
                    "scripts/driver.py".to_string(),
                    "from effect import execute as read_effect\nEXECUTORS = {'Read': read_effect}\ndef parse(value):\n    return value.strip()\ndef drive(value):\n    parsed = parse(value)\n    return EXECUTORS['Read'](parsed)\n".to_string(),
                ),
                (
                    "scripts/effect.py".to_string(),
                    "from pathlib import Path\ndef execute(path):\n    return Path(path).read_text()\n".to_string(),
                ),
            ]),
            semantic_functions: vec![
                SemanticFunctionExpectation {
                    id: "driver".to_string(),
                    symbol: "scripts/driver.py#drive".to_string(),
                    purity: "effectful".to_string(),
                    authorities: BTreeSet::from(["operator".to_string()]),
                },
                SemanticFunctionExpectation {
                    id: "effect".to_string(),
                    symbol: "scripts/effect.py#execute".to_string(),
                    purity: "effectful".to_string(),
                    authorities: BTreeSet::from(["operator".to_string()]),
                },
                SemanticFunctionExpectation {
                    id: "parser".to_string(),
                    symbol: "scripts/driver.py#parse".to_string(),
                    purity: "pure".to_string(),
                    authorities: BTreeSet::new(),
                },
            ],
            authority_facades: vec![AuthorityFacade {
                authority: "operator".to_string(),
                symbol: "scripts/driver.py#drive".to_string(),
            }],
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result
            .functions
            .iter()
            .all(|function| function.unresolved_calls.is_empty()));
    }

    #[test]
    fn swift_local_calls_resolve() {
        let result = report(
            "swift",
            "Sources/App.swift",
            "func helper(_ value: Int) -> Int { value + 1 }\nfunc decide(_ value: Int) -> Int { helper(value) }",
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn swift_provider_named_helpers_do_not_imply_provider_authority() {
        let result = report(
            "swift",
            "Sources/App.swift",
            "func providerCommand() -> Int { 1 }\nfunc providerBranch() -> Int { providerCommand() }\nfunc decide() -> Int { providerBranch() }",
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.is_empty());
    }

    #[test]
    fn swift_readiness_and_resolve_domain_calls_do_not_imply_filesystem_authority() {
        let result = report(
            "swift",
            "Sources/App.swift",
            r#"
                enum Event {
                    case readinessProjected(Int)
                }

                enum Machine {
                    static func resolve(_ value: Int) -> Int { value }
                }

                func decide(_ value: Int) -> (Event, Int) {
                    (.readinessProjected(value), Machine.resolve(value))
                }
            "#,
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.is_empty());
    }

    #[test]
    fn python_file_methods_remain_filesystem_authority() {
        for method in ["open", "read", "read_text", "resolve"] {
            let source = format!("def decide(path):\n    return path.{method}()\n");
            let result = report(
                "python",
                "src/app.py",
                &source,
                expectation("src/app.py#decide", "effectful", &["filesystem"]),
            );
            assert_eq!(result.result, AnalysisResult::Pass, "{method}: {result:#?}");
            assert_eq!(
                result.functions[0].transitive_authorities,
                vec!["filesystem"]
            );
        }
    }

    #[test]
    fn declared_provider_facade_still_implies_provider_authority() {
        let result = analyze(AnalysisInput {
            binding: "js".to_string(),
            source_digest: "source".to_string(),
            tool_digest: "tool".to_string(),
            sources: BTreeMap::from([(
                "src/index.js".to_string(),
                "export function decide() { return provider.complete(); }".to_string(),
            )]),
            semantic_functions: vec![expectation(
                "src/index.js#decide",
                "effectful",
                &["provider"],
            )],
            authority_facades: vec![AuthorityFacade {
                authority: "provider".to_string(),
                symbol: "complete".to_string(),
            }],
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert_eq!(result.functions[0].transitive_authorities, vec!["provider"]);
    }

    #[test]
    fn swift_standard_collection_subscripts_stay_pure() {
        let source = r#"
            let retryDelays: [UInt64] = [1_000, 2_000]

            struct Context {
                var inFlight: [String: Int]
            }

            func decide(
                _ input: [Int],
                context: Context,
                index: Int
            ) -> Int {
                var ordered = input
                var byLane: [String: Int] = [:]
                byLane["direct"] = ordered[index]
                return byLane["direct"]
                    ?? context.inFlight["direct"]
                    ?? Int(retryDelays[index])
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn swift_unknown_subscripts_fail_closed() {
        let source = r#"
            struct Lookup {
                subscript(index: Int) -> Int { index }
            }

            func decide(_ lookup: Lookup, index: Int) -> Int {
                lookup[index]
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
        assert_eq!(result.functions[0].unresolved_calls, vec!["lookup"]);
    }

    #[test]
    fn swift_standard_value_methods_stay_pure_without_filesystem_false_positives() {
        let source = r#"
            struct Context {
                var proofs: [Int]
                var failedLanes: Set<String>
                var candidate: Int?

                mutating func decide() -> Int? {
                    proofs.removeAll { $0 < 0 }
                    failedLanes.formUnion(["direct"])
                    return candidate.flatMap { value in
                        proofs.firstIndex(of: value)
                    }
                }
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#Context::decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.is_empty());
    }

    #[test]
    fn swift_standard_string_collection_join_is_pure() {
        let result = report(
            "swift",
            "Sources/App.swift",
            "func stableDigest(_ components: [String]) -> String { components.joined(separator: \"|\") }",
            expectation("Sources/App.swift#stableDigest", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.is_empty());
    }

    #[test]
    fn swift_standard_zip_operator_order_check_is_pure() {
        let source = r#"
            func strictlyIncreasing(_ values: [UInt64]) -> Bool {
                zip(values, values.dropFirst()).allSatisfy(<)
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#strictlyIncreasing", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.is_empty());
        assert!(result.functions[0].unresolved_calls.is_empty());
    }

    #[test]
    fn swift_enum_array_binding_zip_operator_order_check_is_pure() {
        let source = r#"
            enum Input { case replace(retainedFrameIndices: [UInt64]) }
            func decide(_ input: Input) -> Bool {
                switch input {
                case .replace(let retainedFrameIndices):
                    return zip(retainedFrameIndices, retainedFrameIndices.dropFirst()).allSatisfy(<)
                }
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn swift_standard_all_satisfy_with_open_callback_fails_closed() {
        let source = r#"
            func decide(_ values: [UInt64], predicate: (UInt64) -> Bool) -> Bool {
                values.allSatisfy(predicate)
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
        assert_eq!(
            result.functions[0].transitive_authorities,
            vec!["dynamic-dispatch"]
        );
    }

    #[test]
    fn swift_literal_collection_closures_and_join_stay_pure() {
        let source = r#"
            func decide(_ first: String, _ second: String) -> Bool {
                let values = [first, second].map { $0.trimmingCharacters(in: .whitespaces) }
                let trace = [String(describing: first), second].joined(separator: "|")
                return values.allSatisfy({ !$0.isEmpty }) && !trace.isEmpty
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn swift_standard_numeric_overflow_and_sorted_drop_first_stay_pure() {
        let source = r#"
            func decide(_ lhs: UInt64, _ rhs: UInt64, frames: Set<UInt64>) -> UInt64 {
                let (sum, overflow) = lhs.addingReportingOverflow(rhs)
                let ordered = frames.sorted()
                for frame in ordered.dropFirst() { _ = frame }
                return overflow ? UInt64.max : sum
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn swift_unknown_value_methods_fail_closed() {
        let source = r#"
            func decide(_ store: inout CustomStore) {
                store.removeAll()
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
        assert_eq!(
            result.functions[0].transitive_authorities,
            vec!["dynamic-dispatch"]
        );
    }

    #[test]
    fn swift_unary_local_calls_and_immediate_closures_stay_pure() {
        let source = r#"
            func isEligible(_ value: Int) -> Bool { value > 0 }

            func decide(_ value: Int) -> Bool {
                let closureResult: Bool = {
                    !isEligible(value)
                }()
                return closureResult
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert_eq!(result.functions[0].resolved_callees, vec!["isEligible"]);
    }

    #[test]
    fn swift_immediate_closure_effects_remain_visible() {
        let source = r#"
            func decide(_ url: URL) -> Bool {
                {
                    URLSession.shared.dataTask(with: url)
                    return true
                }()
            }
        "#;
        let result = report(
            "swift",
            "Sources/App.swift",
            source,
            expectation("Sources/App.swift#decide", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
        assert_eq!(result.functions[0].transitive_authorities, vec!["network"]);
    }

    #[test]
    fn javascript_fs_namespace_remains_filesystem_authority() {
        let result = report(
            "javascript",
            "src/app.js",
            "function decide(path) { return fs.readFile(path); }",
            expectation("src/app.js#decide", "effectful", &["filesystem"]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn rms_functional_cores_have_no_dynamic_calls() {
        for (path, source) in [
            (
                "src/binding_migration.rs",
                include_str!("binding_migration.rs"),
            ),
            (
                "src/schema_generator.rs",
                include_str!("schema_generator.rs"),
            ),
            (
                "src/composition_model.rs",
                include_str!("composition_model.rs"),
            ),
        ] {
            let dynamic = extract_rust_functions(path, source)
                .into_iter()
                .filter(|node| node.calls.contains("<dynamic-call>"))
                .map(|node| node.qualified_name)
                .collect::<Vec<_>>();
            assert!(dynamic.is_empty(), "{path} dynamic nodes: {dynamic:?}");
        }
    }

    #[test]
    fn swift_transition_property_projection_stays_pure() {
        let source = r#"
            public struct Output { public let value: Int }
            public struct Record { public let output: Output }
            public enum Machine {
                public static func transition(_ value: Int) -> Output {
                    transitionRecord(value).output
                }
            }
            public func transition(_ value: Int) -> Output {
                transitionRecord(value).output
            }
            public func transitionRecord(_ value: Int) -> Record {
                Record(output: Output(value: value))
            }
        "#;
        let result = report(
            "swift",
            "Sources/Example/Transition.swift",
            source,
            expectation("Sources/Example/Transition.swift#transition", "pure", &[]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn swift_callable_selector_resolves_one_exact_overload() {
        let source = r#"
            public actor Facade {
                public func send(_ command: Command) async -> Result { Result() }
                public func send(_ envelope: Envelope) async -> Result { Result() }
            }
        "#;
        let selected = report(
            "swift",
            "Sources/Facade.swift",
            source,
            expectation("Sources/Facade.swift#Facade.send(_:Envelope)", "pure", &[]),
        );
        assert_eq!(selected.result, AnalysisResult::Pass, "{selected:#?}");

        let ambiguous = report(
            "swift",
            "Sources/Facade.swift",
            source,
            expectation("Sources/Facade.swift#Facade.send", "pure", &[]),
        );
        assert_eq!(ambiguous.result, AnalysisResult::Fail, "{ambiguous:#?}");

        let missing = report(
            "swift",
            "Sources/Facade.swift",
            source,
            expectation("Sources/Facade.swift#Facade.send(_:Missing)", "pure", &[]),
        );
        assert_eq!(missing.result, AnalysisResult::Fail, "{missing:#?}");
    }
    #[test]
    fn swift_defer_keyword_is_not_a_call_but_its_body_is_traversed() {
        let result = report(
            "swift",
            "Sources/Facade.swift",
            r#"
                func releaseAuthority() { FileManager.default.remove_file("token") }
                func perform() {
                    defer { releaseAuthority() }
                }
            "#,
            expectation("Sources/Facade.swift#perform", "effectful", &["filesystem"]),
        );
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(!result.functions[0]
            .unresolved_calls
            .contains(&"defer".to_string()));
        assert!(result.functions[0]
            .resolved_callees
            .contains(&"releaseAuthority".to_string()));
    }
}

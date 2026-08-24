use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use syn::visit::{self, Visit};
use syn::{
    Expr, ExprCall, ExprMacro, ExprMethodCall, FnArg, ImplItemFn, ItemFn, ItemImpl, ItemMod, Local,
    Pat, Type, UseTree,
};
use tree_sitter::{Language, Node, Parser};

pub(crate) const EFFECT_ANALYSIS_SPEC: &str = "rms/effect-analysis/v0.1";
pub(crate) const PURE_ALLOWLIST_VERSION: &str = "rms/pure-call-allowlist/v0.1";
pub(crate) const AUTHORITY_ROOT_VERSION: &str = "rms/authority-root-allowlist/v0.1";

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
    calls: BTreeSet<String>,
    direct_authorities: BTreeSet<String>,
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
    let mut nodes = Vec::new();
    for (path, source) in &input.sources {
        let source_binding = if input.binding == "executable" {
            source_binding(path).unwrap_or("")
        } else {
            input.binding.as_str()
        };
        let mut extracted = if source_binding == "rust" {
            extract_rust_functions(path, source)
        } else {
            extract_tree_sitter_functions(source_binding, path, source, &swift_global_names)
        };
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
    let expected_path = symbol_path(symbol);
    let mut candidates = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node.name == expected_name
                && expected_path.is_none_or(|path| normalized_path_matches(&node.path, path))
                && (!expected_qualified.contains("::") || node.qualified_name == expected_qualified)
        })
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
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
) {
    if !visited.insert(index) {
        return;
    }
    let node = &nodes[index];
    authorities.extend(authorities_for_node(index, nodes, facades));
    for call in &node.calls {
        let candidates = resolve_local_call(index, call, nodes);
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
            );
            continue;
        }
        if let Some(authority) = authority_for_call(&node.binding, call)
            .or_else(|| facades.get(symbol_name(call)).cloned())
        {
            authorities.insert(authority);
            continue;
        }
        if known_pure_call(call)
            || swift_standard_value_method(call, &node.swift_standard_value_names)
        {
            continue;
        }
        if candidates.len() == 1 {
            resolved.insert(nodes[candidates[0]].qualified_name.clone());
            collect_closure(
                candidates[0],
                nodes,
                facades,
                visited,
                resolved,
                unresolved,
                authorities,
            );
        } else if call_is_constructor(call) {
            continue;
        } else if node.binding == "shell" {
            unresolved.insert(call.clone());
        } else if call == "<dynamic-call>" || call.contains('.') {
            authorities.insert("dynamic-dispatch".to_string());
            resolved.insert(call.clone());
        } else {
            unresolved.insert(call.clone());
        }
    }
}

fn resolve_local_call(index: usize, call: &str, nodes: &[FunctionNode]) -> Vec<usize> {
    let name = symbol_name(call);
    let expected_path = symbol_path(call);
    let same_file_free = nodes
        .iter()
        .enumerate()
        .filter(|(_, candidate)| {
            candidate.path == nodes[index].path
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
        .filter(|candidate| nodes[*candidate].qualified_name == requested_qualified)
        .collect::<Vec<_>>();
    if exact.len() == 1 {
        return exact;
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
    Vec::new()
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
        if node.binding == "shell" && resolve_local_call(index, call, nodes).len() == 1 {
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
    let compact = call.replace(' ', "");
    let lower = compact.to_ascii_lowercase();
    let authority = if [
        "std::fs",
        "file::open",
        ".open",
        ".read_bytes",
        ".read_text",
        ".read",
        ".write_bytes",
        ".write_text",
        ".mkdir",
        ".resolve",
        "read_to_string",
        "write_file",
        "readfile",
        "writefile",
        "pathlib.",
        "walkdir",
        "canonicalize",
        "create_dir",
        "metadata",
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
        || path_contains_segment(&lower, "fs")
    {
        "filesystem"
    } else if [
        "command::",
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
    } else if binding != "swift" && (lower.contains("provider") || lower.contains("codex")) {
        "provider"
    } else {
        return None;
    };
    Some(authority.to_string())
}

fn known_pure_call(call: &str) -> bool {
    let name = symbol_name(call).trim_end_matches('!');
    matches!(
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
            | "as_ref"
            | "as_sequence"
            | "as_sequence_mut"
            | "as_slice"
            | "as_str"
            | "as_u64"
            | "at"
            | "bool"
            | "borrow"
            | "borrow_mut"
            | "byte_range"
            | "chain"
            | "char_indices"
            | "chars"
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
            | "to_le_bytes"
            | "to_lowercase"
            | "toLowerCase"
            | "to_os_string"
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
            | "wrapping_add"
            | "zip"
    ) || call.starts_with("serde_json::to_")
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
        "firstIndex" | "flatMap" | "formUnion" | "removeAll" | "removeValue"
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
        || call
            .split([':', '.'])
            .find(|part| !part.is_empty())
            .and_then(|root| root.chars().next())
            .is_some_and(char::is_uppercase)
        || symbol_name(call)
            .chars()
            .next()
            .is_some_and(char::is_uppercase)
}

fn symbol_name(symbol: &str) -> &str {
    symbol
        .rsplit_once('#')
        .map_or(symbol, |(_, name)| name)
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
        .map_or(symbol, |(_, name)| name)
        .replace('.', "::")
}

fn normalized_path_matches(actual: &str, expected: &str) -> bool {
    actual
        .replace('\\', "/")
        .ends_with(&expected.replace('\\', "/"))
}

#[derive(Default)]
struct RustCallCollector {
    calls: BTreeSet<String>,
    authorities: BTreeSet<String>,
    dynamic_symbols: BTreeSet<String>,
    local_closures: BTreeSet<String>,
}

impl<'ast> Visit<'ast> for RustCallCollector {
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
            if self.local_closures.contains(leaf) {
                visit::visit_expr_call(self, node);
                return;
            }
            if self.dynamic_symbols.contains(leaf) {
                self.calls.insert("<dynamic-call>".to_string());
                visit::visit_expr_call(self, node);
                return;
            }
            if let Some(authority) = authority_for_call("rust", &call) {
                self.authorities.insert(authority);
            }
            self.calls.insert(call);
        } else {
            self.calls.insert("<dynamic-call>".to_string());
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let receiver = rust_expr_label(&node.receiver);
        let call = if receiver.is_empty() {
            node.method.to_string()
        } else {
            format!("{receiver}.{}", node.method)
        };
        if rust_expr_root_ident(&node.receiver)
            .is_some_and(|root| self.dynamic_symbols.contains(&root))
            && !known_pure_call(&call)
        {
            self.calls.insert("<dynamic-call>".to_string());
            visit::visit_expr_method_call(self, node);
            return;
        }
        if let Some(authority) = authority_for_call("rust", &call) {
            self.authorities.insert(authority);
        }
        self.calls.insert(call);
        visit::visit_expr_method_call(self, node);
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
        if node
            .init
            .as_ref()
            .is_some_and(|init| matches!(init.expr.as_ref(), Expr::Closure(_)))
        {
            if let Pat::Ident(ident) = &node.pat {
                self.local_closures.insert(ident.ident.to_string());
            }
        }
        visit::visit_local(self, node);
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
}

impl<'ast> Visit<'ast> for RustFunctionCollector {
    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if self.test_depth > 0 || has_test_attribute(&node.attrs) {
            return;
        }
        let mut calls = RustCallCollector {
            dynamic_symbols: rust_dynamic_parameters(node.sig.inputs.iter()),
            ..RustCallCollector::default()
        };
        calls.visit_block(&node.block);
        let name = node.sig.ident.to_string();
        self.nodes.push(FunctionNode {
            binding: "rust".to_string(),
            path: self.path.clone(),
            qualified_name: qualified_rust_name(&self.owner, &name),
            name,
            calls: calls.calls,
            direct_authorities: calls.authorities,
            swift_standard_value_names: BTreeSet::new(),
        });
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        if self.test_depth > 0 || has_test_attribute(&node.attrs) {
            return;
        }
        let mut calls = RustCallCollector {
            dynamic_symbols: rust_dynamic_parameters(node.sig.inputs.iter()),
            ..RustCallCollector::default()
        };
        calls.visit_block(&node.block);
        let name = node.sig.ident.to_string();
        self.nodes.push(FunctionNode {
            binding: "rust".to_string(),
            path: self.path.clone(),
            qualified_name: qualified_rust_name(&self.owner, &name),
            name,
            calls: calls.calls,
            direct_authorities: calls.authorities,
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

fn qualified_rust_name(owner: &[String], name: &str) -> String {
    if owner.is_empty() {
        name.to_string()
    } else {
        format!("{}::{name}", owner.join("::"))
    }
}

fn extract_rust_functions(path: &str, source: &str) -> Vec<FunctionNode> {
    let Ok(file) = syn::parse_file(source) else {
        return Vec::new();
    };
    let mut collector = RustFunctionCollector {
        nodes: Vec::new(),
        path: path.to_string(),
        owner: Vec::new(),
        test_depth: 0,
    };
    collector.visit_file(&file);
    let aliases = rust_import_aliases(&file);
    for node in &mut collector.nodes {
        node.calls = node
            .calls
            .iter()
            .map(|call| resolve_call_alias(call, &aliases))
            .collect();
    }
    collector.nodes
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
                calls,
                direct_authorities,
                swift_standard_value_names,
            })
        })
        .collect()
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
            let resolved = normalized_import_path(path, &module.replace('.', "/"));
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
                aliases.insert(alias.to_string(), format!("{resolved}.py#{original}"));
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
        if binding == "swift" && swift_call_is_immediate_closure(callee, source) {
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
    text.contains("Array<") || text.contains("Dictionary<")
}

fn has_standard_value_type(node: Node<'_>, source: &str) -> bool {
    if has_standard_collection_type(node, source) || node_has_kind(node, "optional_type") {
        return true;
    }
    let Ok(text) = node.utf8_text(source.as_bytes()) else {
        return false;
    };
    text.contains("Set<") || text.contains("Optional<")
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
}

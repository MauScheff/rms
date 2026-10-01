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
pub(crate) const PURE_ALLOWLIST_VERSION: &str = "rms/pure-call-allowlist/v0.8";
pub(crate) const AUTHORITY_ROOT_VERSION: &str = "rms/authority-root-allowlist/v0.6";

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
    let mut facades = BTreeMap::<String, BTreeSet<String>>::new();
    for facade in &input.authority_facades {
        let key = if input.binding == "rust" { facade.symbol.clone() } else { symbol_name(&facade.symbol).to_string() };
        facades.entry(key).or_default().insert(facade.authority.clone());
    }
    let authority_memberships = authority_memberships(&nodes, &input.authority_facades, &input.semantic_functions);
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
    facades: &BTreeMap<String, BTreeSet<String>>,
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
    let ambient_memberships = memberships.iter().filter(|authority| authority.as_str() != "dynamic-dispatch").collect::<Vec<_>>();
    if ambient_memberships.len() == 1 && !is_raw_authority(ambient_memberships[0]) {
        let authority = ambient_memberships[0];
        direct_authorities = bind_ambient_authorities(direct_authorities, &authority);
        transitive_authorities = bind_ambient_authorities(transitive_authorities, &authority);
    }

    let mut reasons = Vec::new();
    if ambient_memberships.len() > 1 && ambient_memberships.iter().any(|authority| !is_raw_authority(authority)) {
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
    let normalized = format!("selector.swift#{symbol}");
    let symbol = if symbol.contains('#') { symbol } else { &normalized };
    let path = symbol_path(symbol).unwrap_or("selector.swift");
    let nodes =
        extract_tree_sitter_functions("swift", path, source, &SwiftStandardNames::default());
    symbol_candidates(symbol, &nodes).len() == 1
}

pub(crate) fn swift_exact_callable_source<'a>(source: &'a str, symbol: &str) -> Option<&'a str> {
    let normalized = format!("selector.swift#{symbol}");
    let symbol = if symbol.contains('#') { symbol } else { &normalized };
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_swift::LANGUAGE.into()).ok()?;
    let tree = parser.parse(source, None)?;
    let mut declarations = Vec::new();
    collect_function_nodes(tree.root_node(), &mut declarations);
    let path = symbol_path(symbol).unwrap_or("selector.swift");
    let entries = declarations.into_iter().filter_map(|node| {
        let name = function_node_name(node, source)?;
        Some((FunctionNode {
            qualified_name: tree_sitter_qualified_name(node, source, &name),
            name,
            path: path.to_string(),
            callable_selector: swift_callable_selector(node, source),
            ..FunctionNode::default()
        }, node.byte_range()))
    }).collect::<Vec<_>>();
    let nodes = entries.iter().map(|(node, _)| node.clone()).collect::<Vec<_>>();
    let candidates = symbol_candidates(symbol, &nodes);
    if candidates.len() != 1 { return None; }
    source.get(entries[candidates[0]].1.clone())
}

fn authority_memberships(
    nodes: &[FunctionNode],
    facades: &[AuthorityFacade],
    semantic_functions: &[SemanticFunctionExpectation],
) -> BTreeMap<usize, BTreeSet<String>> {
    let mut memberships = BTreeMap::<usize, BTreeSet<String>>::new();
    let facade_roots = facades.iter().flat_map(|facade| symbol_candidates(&facade.symbol, nodes))
        .collect::<BTreeSet<_>>();
    for facade in facades {
        let roots = symbol_candidates(&facade.symbol, nodes);
        if roots.len() != 1 {
            continue;
        }
        let mut closure = BTreeSet::new();
        let mut boundaries = facade_roots.clone();
        boundaries.extend(semantic_functions.iter()
            .filter(|function| !function.authorities.contains(&facade.authority))
            .flat_map(|function| symbol_candidates(&function.symbol, nodes)));
        boundaries.remove(&roots[0]);
        collect_local_members(roots[0], nodes, &mut closure, &boundaries);
        for index in closure {
            memberships
                .entry(index)
                .or_default()
                .insert(facade.authority.clone());
        }
    }
    memberships
}

fn collect_local_members(index: usize, nodes: &[FunctionNode], members: &mut BTreeSet<usize>, boundaries: &BTreeSet<usize>) {
    if boundaries.contains(&index) || !members.insert(index) {
        return;
    }
    for call in &nodes[index].calls {
        let candidates = resolve_local_call(index, call, nodes);
        if candidates.len() == 1 {
            collect_local_members(candidates[0], nodes, members, boundaries);
        }
    }
}

fn bind_ambient_authorities(
    authorities: BTreeSet<String>,
    facade_authority: &str,
) -> BTreeSet<String> {
    let has_ambient = authorities
        .iter()
        .any(|authority| is_raw_authority(authority) && authority != "dynamic-dispatch");
    let mut bound = authorities
        .into_iter()
        .filter(|authority| !is_raw_authority(authority) || authority == "dynamic-dispatch")
        .collect::<BTreeSet<_>>();
    if has_ambient {
        bound.insert(facade_authority.to_string());
    }
    bound
}

pub(crate) fn is_raw_authority(authority: &str) -> bool {
    matches!(authority, "filesystem" | "process" | "clock" | "randomness" | "environment" | "network" | "git" | "unsafe" | "dynamic-dispatch")
}

#[allow(clippy::too_many_arguments)]
fn collect_closure(
    index: usize,
    nodes: &[FunctionNode],
    facades: &BTreeMap<String, BTreeSet<String>>,
    visited: &mut BTreeSet<usize>,
    resolved: &mut BTreeSet<String>,
    unresolved: &mut BTreeSet<String>,
    authorities: &mut BTreeSet<String>,
    trusted_external_calls: &BTreeSet<String>,
) {
    if !visited.insert(index) {
        return;
    }
    let parent_authorities = authorities;
    let mut local_authorities = BTreeSet::new();
    let authorities = &mut local_authorities;
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
        if let Some(authority) = authority_for_call(&node.binding, call) {
            authorities.insert(authority);
            continue;
        }
        if let Some(required) = external_facade_authorities(call, facades) {
            authorities.extend(required.iter().cloned());
            // A named facade contains ambient IO. It does not erase open
            // dispatch or unresolved calls in an inspectable implementation.
            let mut contained = BTreeSet::new();
            let mut facade_visited = visited.clone();
            for candidate in candidates {
                resolved.insert(nodes[candidate].qualified_name.clone());
                collect_closure(candidate, nodes, facades, &mut facade_visited,
                    resolved, unresolved, &mut contained, trusted_external_calls);
            }
            let raw_only = required.iter().all(|authority| is_raw_authority(authority));
            authorities.extend(contained.into_iter().filter(|authority| raw_only || authority == "dynamic-dispatch"));
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
    let required = exact_node_facade_authorities(index, nodes, facades);
    let ambient = required.iter().filter(|authority| authority.as_str() != "dynamic-dispatch").collect::<Vec<_>>();
    if ambient.len() == 1 && !is_raw_authority(ambient[0]) {
        local_authorities = bind_ambient_authorities(local_authorities, ambient[0]);
    }
    parent_authorities.extend(local_authorities);
}

fn exact_node_facade_authorities(index: usize, nodes: &[FunctionNode], facades: &BTreeMap<String, BTreeSet<String>>) -> BTreeSet<String> {
    facades.iter().filter(|(symbol, _)| symbol_candidates(symbol, nodes) == vec![index])
        .flat_map(|(_, authorities)| authorities.iter().cloned()).collect()
}

fn external_facade_authorities<'a>(call: &str, facades: &'a BTreeMap<String, BTreeSet<String>>) -> Option<&'a BTreeSet<String>> {
    // Only explicitly unqualified external facade declarations may match by leaf.
    // A source-qualified facade must resolve to its exact source callable.
    facades.get(call).filter(|_| !call.contains('#'))
        .or_else(|| facades.get(symbol_name(call)).filter(|_| !symbol_name(call).contains('#')))
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
    if nodes[index].binding == "rust" {
        if let Some(path) = expected_path {
            let qualified = symbol_qualified_name(call);
            let has_exact_path = nodes.iter().any(|candidate| candidate.path == path);
            let exact = nodes.iter().enumerate().filter(|(_, candidate)|
                (if has_exact_path { candidate.path == path } else { normalized_path_matches(&candidate.path, path) })
                    && candidate.qualified_name == qualified)
                .map(|(index, _)| index).collect::<Vec<_>>();
            // An exact source identity never falls back to a same-leaf helper.
            return if exact.len() == 1 { exact } else { Vec::new() };
        }
    }
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
    if nodes[index].binding == "rust" && call.split_once("::")
        .is_some_and(|(owner, _)| owner.chars().next().is_some_and(char::is_uppercase)) {
        // A named receiver is not interchangeable with another type (or a
        // free function) merely because their method leaves match.
        let exact = direct.iter().copied().filter(|candidate|
            nodes[*candidate].qualified_name == symbol_qualified_name(call))
            .collect::<Vec<_>>();
        return if exact.len() == 1 { exact } else { Vec::new() };
    }
    if direct.len() == 1 {
        return direct;
    }
    let requested_qualified = symbol_qualified_name(call);
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
    facades: &BTreeMap<String, BTreeSet<String>>,
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
        if let Some(authority) = authority_for_call(&node.binding, call) {
            authorities.insert(authority);
        } else if let Some(required) = external_facade_authorities(call, facades) {
            authorities.extend(required.iter().cloned());
        }
    }
    authorities
}

fn authority_for_call(binding: &str, call: &str) -> Option<String> {
    if binding == "rust" && call == "<rust-git2-call>" { return Some("git".into()); }
    if binding == "rust" && call == "<rust-stdin-read>" { return Some("filesystem".into()); }
    if binding == "rust" && call == "<rust-external-uuid-v4>" {
        return Some("randomness".into());
    }
    if binding == "python" {
        match call {
            "python-stdlib.urllib.request.urlopen" => return Some("network".to_string()),
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
    if call == "<rust-sequence-retain>" { return true; }
    if call == "<rust-character-predicate>" { return true; }
    if call == "<rust-io-error-kind>" { return true; }
    if call == "<rust-byte-ascii-hexdigit>" { return true; }
    if call == "<rust-text-ascii-equality>" { return true; }
    if matches!(call, "<rust-str-literal-split-inclusive>" | "<rust-yaml-value-from-str>") { return true; }
    if call == "<rust-integer-byte-conversion>" { return true; }
    if call == "<rust-sequence-last-mut>" { return true; }
    if call == "<rust-standard-poll-fn>" { return true; }
    if matches!(call, "<rust-iterator-max-by-key>" | "<rust-str-ascii-uppercase>") { return true; }
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
    matched_value: Option<RustValueType>,
    sequence_constraints: BTreeMap<String, RustValueType>,
    standard_str: bool,
    return_error: Option<RustValueType>,
}

impl<'ast> Visit<'ast> for RustCallCollector {
    fn visit_expr_struct(&mut self, node: &'ast syn::ExprStruct) {
        if node.qself.is_some() || !self.type_index.git_index_entry(&node.path) {
            visit::visit_expr_struct(self, node);
            return;
        }
        for field in &node.fields {
            let conversion = (|| {
                if !matches!(&field.member, syn::Member::Named(name) if name == "file_size") { return None; }
                let Expr::Try(checked) = &field.expr else { return None; };
                let Expr::MethodCall(mapped) = checked.expr.as_ref() else { return None; };
                if mapped.method != "map_err" || mapped.args.len() != 1 || mapped.turbofish.is_some()
                    || !matches!(&mapped.args[0], Expr::Closure(closure) if closure.inputs.len() == 1) { return None; }
                let Expr::Closure(mapper) = &mapped.args[0] else { return None; };
                if mapper.asyncness.is_some() || self.return_error.is_none()
                    || self.type_index.unit_variant_type(&mapper.body) != self.return_error { return None; }
                let Expr::MethodCall(converted) = mapped.receiver.as_ref() else { return None; };
                if converted.method != "try_into" || !converted.args.is_empty() || converted.turbofish.is_some()
                    || self.type_index.expression_type(&converted.receiver, &self.value_types) != Some(RustValueType::Usize) { return None; }
                Some((converted, mapped))
            })();
            if let Some((converted, mapped)) = conversion {
                // The pinned IndexEntry.file_size is u32. This exact standard
                // usize -> u32 checked conversion cannot invoke user code.
                self.visit_expr(&converted.receiver);
                self.visit_expr(&mapped.args[0]);
            } else { self.visit_expr(&field.expr); }
        }
        if let Some(rest) = &node.rest { self.visit_expr(rest); }
    }

    fn visit_expr_return(&mut self, node: &'ast syn::ExprReturn) {
        let conversion = (|| {
            let Expr::Call(error) = node.expr.as_deref()? else { return None; };
            let Expr::Path(path) = error.func.as_ref() else { return None; };
            if !path.path.is_ident("Err") || error.args.len() != 1 || self.type_index.call_root_shadowed("Err")
                || self.value_types.contains_key("Err") || !self.type_index.unshadowed_external_root("Err") { return None; }
            let Expr::MethodCall(conversion) = &error.args[0] else { return None; };
            if conversion.method != "into" || !conversion.args.is_empty() || conversion.turbofish.is_some() { return None; }
            let source = self.type_index.expression_type(&conversion.receiver, &self.value_types)?;
            self.type_index.closed_error_wrapper(&source, self.return_error.as_ref()?).then_some(conversion)
        })();
        if let Some(conversion) = conversion {
            self.visit_expr(&conversion.receiver);
        } else { visit::visit_expr_return(self, node); }
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        let prior = self.return_error.take();
        visit::visit_item_fn(self, node);
        self.return_error = prior;
    }
    fn visit_expr_if(&mut self, node: &'ast syn::ExprIf) {
        let Expr::Let(condition) = node.cond.as_ref() else {
            visit::visit_expr_if(self, node);
            return;
        };
        self.visit_expr(&condition.expr);
        let prior_values = self.value_types.clone();
        let prior_types = self.parameter_types.clone();
        let prior_dynamic = self.dynamic_symbols.clone();
        let prior_callbacks = self.callback_bindings.clone();
        let mut names = BTreeSet::new();
        collect_rust_pattern_identifiers(&condition.pat, &mut names);
        let value = self.type_index.expression_type(&condition.expr, &self.value_types);
        for name in names {
            self.value_types.insert(name.clone(), RustValueType::Unknown);
            self.parameter_types.remove(&name);
            self.callback_bindings.remove(&name);
            self.dynamic_symbols.insert(name);
        }
        if let Some(value) = value {
            for (name, value) in self.type_index.pattern_bindings(&condition.pat, &value) {
                if let Some(ty) = value.name() {
                    self.parameter_types.insert(name.clone(), ty.clone());
                    self.dynamic_symbols.remove(&name);
                }
                self.value_types.insert(name, value);
            }
        }
        self.visit_block(&node.then_branch);
        self.value_types = prior_values;
        self.parameter_types = prior_types;
        self.dynamic_symbols = prior_dynamic;
        self.callback_bindings = prior_callbacks;
        if let Some((_, otherwise)) = &node.else_branch { self.visit_expr(otherwise); }
    }

    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        self.visit_expr(&node.expr);
        let prior = self.matched_value.take();
        self.matched_value = self.type_index.expression_type(&node.expr, &self.value_types);
        for arm in &node.arms { self.visit_arm(arm); }
        self.matched_value = prior;
    }

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
            Some(RustValueType::Sequence(element) | RustValueType::Iterator(element) | RustValueType::Set(element)) => {
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
        if let Some(value) = &self.matched_value {
            for (name, value) in self.type_index.pattern_bindings(&node.pat, value) {
                if let Some(ty) = value.name() {
                    self.parameter_types.insert(name.clone(), ty.clone());
                    self.dynamic_symbols.remove(&name);
                } else if value == RustValueType::Unknown {
                    self.dynamic_symbols.insert(name.clone());
                }
                self.value_types.insert(name, value);
            }
        }
        visit::visit_arm(self, node);
        self.value_types = prior_values;
        self.parameter_types = prior_types;
        self.dynamic_symbols = prior_dynamic;
        self.callback_bindings = prior_callbacks;
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        let prior_index = self.type_index.clone();
        self.type_index = self.type_index.with_local_type_shadows(block);
        let prior_values = self.value_types.clone();
        let prior_types = self.parameter_types.clone();
        let prior_dynamic = self.dynamic_symbols.clone();
        let prior_callbacks = self.callback_bindings.clone();
        let prior_closures = self.local_closures.clone();
        let prior_constraints = self.sequence_constraints.clone();
        for statement in &block.stmts {
            let syn::Stmt::Local(local) = statement else { continue; };
            let Pat::Ident(name) = &local.pat else { continue; };
            self.sequence_constraints.remove(&name.ident.to_string());
            if local.init.as_ref().is_some_and(|init| self.type_index.is_standard_empty_vec(&init.expr)) {
                if let Some(value) = self.type_index.empty_sequence_constraint(&name.ident.to_string(), block, &self.value_types) {
                    self.sequence_constraints.insert(name.ident.to_string(), value);
                }
            }
            if local.init.as_ref().is_some_and(|init| self.type_index.is_standard_empty_map(&init.expr)) {
                if let Some(value) = self.type_index.empty_map_constraint(&name.ident.to_string(), block, &self.value_types) {
                    self.sequence_constraints.insert(name.ident.to_string(), value);
                }
            }
        }
        visit::visit_block(self, block);
        self.value_types = prior_values;
        self.parameter_types = prior_types;
        self.dynamic_symbols = prior_dynamic;
        self.callback_bindings = prior_callbacks;
        self.local_closures = prior_closures;
        self.sequence_constraints = prior_constraints;
        self.type_index = prior_index;
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        let mut callee = node.func.as_ref();
        loop {
            callee = match callee {
                Expr::Paren(expr) => &expr.expr,
                Expr::Group(expr) => &expr.expr,
                _ => break,
            };
        }
        if matches!(callee, Expr::Closure(_)) {
            // The body is known. Retain its effects and unknown nested calls.
            visit::visit_expr_call(self, node);
            return;
        }
        if self.is_yaml_value_parse(node, false) {
            self.calls.insert("<rust-yaml-value-from-str>".into());
            visit::visit_expr_call(self, node);
            return;
        }
        if let syn::Expr::Path(path) = node.func.as_ref() {
            if self.type_index.local_enum_constructor(path, node.args.len())
                && path.path.segments.first().is_some_and(|root|
                    !self.value_types.contains_key(&root.ident.to_string())
                        && !self.dynamic_symbols.contains(&root.ident.to_string()))
            {
                // Constructing a proven variant is pure; its arguments may not be.
                visit::visit_expr_call(self, node);
                return;
            }
            if path.path.leading_colon.is_none() && path.path.segments.first().is_some_and(|root|
                (self.type_index.call_root_shadowed(&root.ident.to_string())
                    || self.value_types.contains_key(&root.ident.to_string()))
                    && !self.callback_bindings.contains_key(&root.ident.to_string())
                    && !self.local_closures.contains(&root.ident.to_string()))
            {
                self.calls.insert("<dynamic-call>".into());
                visit::visit_expr_call(self, node);
                return;
            }
            if node.args.is_empty() && self.type_index.external_uuid_v4(path)
                && path.path.segments.first().is_some_and(|root|
                    !self.value_types.contains_key(&root.ident.to_string())
                        && !self.dynamic_symbols.contains(&root.ident.to_string()))
            {
                self.calls.insert("<rust-external-uuid-v4>".into());
                visit::visit_expr_call(self, node);
                return;
            }
            let call = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            let leaf = symbol_name(&call);
            if path.path.segments.len() == 2 && node.args.len() == 1
                && matches!(leaf, "from_be_bytes" | "from_le_bytes" | "from_ne_bytes" | "to_be_bytes" | "to_le_bytes" | "to_ne_bytes")
                && self.type_index.standard_integer_available(&path.path.segments[0].ident.to_string())
            {
                self.calls.insert("<rust-integer-byte-conversion>".into());
                visit::visit_expr_call(self, node);
                return;
            }
            if path.path.leading_colon.is_some()
                && matches!(call.as_str(), "std::future::poll_fn" | "core::future::poll_fn")
                && node.args.len() == 1
                && matches!(node.args.first(), Some(Expr::Closure(closure)) if closure.inputs.len() == 1)
                && self.type_index.standard_external_crate_available(&path.path.segments[0].ident.to_string())
            {
                self.calls.insert("<rust-standard-poll-fn>".into());
                // poll_fn supplies scheduling, not an authority exemption. All
                // callback calls and captured dynamic dispatch remain visible.
                visit::visit_expr_call(self, node);
                return;
            }
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
                        standard_str: self.type_index.standard_str_available(&helper.sig, &helper.block),
                        parameter_types: rust_parameter_types(helper.sig.inputs.iter()),
                        value_types: self.type_index.parameters(&helper.sig),
                        type_index: self.type_index.with_generics(&helper.sig.generics),
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
            self.calls.insert(if self.standard_str && call == "str::to_ascii_uppercase" {
                "<rust-str-ascii-uppercase>".to_string()
            } else { call });
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
        let external_marker = inferred_receiver.as_ref().and_then(|value| {
            node.turbofish.is_none().then(|| self.type_index.external_method_marker(value, &node.method.to_string(), node.args.len())).flatten()
        });
        if matches!(inferred_receiver, Some(RustValueType::External(_))) && external_marker.is_none() {
            self.calls.insert("<dynamic-call>".into());
            visit::visit_expr_method_call(self, node);
            return;
        }
        let typed_receiver = inferred_name.or_else(|| match node.receiver.as_ref() {
            Expr::Path(path) if path.path.segments.len() == 1 => self
                .parameter_types
                .get(&path.path.segments[0].ident.to_string())
                .filter(|name| !self.type_index.is_generic(name) && !self.type_index.is_declared_type(name)),
            _ => None,
        });
        let call = if let Some(marker) = external_marker {
            marker.to_string()
        } else if inferred_receiver == Some(RustValueType::Character)
            && matches!(node.method.to_string().as_str(), "is_control" | "is_whitespace")
            && node.args.is_empty() && node.turbofish.is_none() {
            // Only a proven standard character receiver closes this call.
            // The enclosing iterator callback is still analyzed in full.
            "<rust-character-predicate>".to_string()
        } else if inferred_receiver == Some(RustValueType::Byte)
            && node.method == "is_ascii_hexdigit" && node.args.is_empty() && node.turbofish.is_none() {
            "<rust-byte-ascii-hexdigit>".to_string()
        } else if inferred_receiver == Some(RustValueType::Text)
            && node.method == "eq_ignore_ascii_case" && node.args.len() == 1 {
            "<rust-text-ascii-equality>".to_string()
        } else if inferred_receiver == Some(RustValueType::Text)
            && node.method == "split_inclusive" && node.args.len() == 1
            && matches!(&node.args[0], Expr::Lit(value) if matches!(value.lit, syn::Lit::Char(_) | syn::Lit::Str(_))) {
            "<rust-str-literal-split-inclusive>".to_string()
        } else if node.args.is_empty()
            && matches!(node.method.to_string().as_str(), "to_be_bytes" | "to_le_bytes" | "to_ne_bytes")
            && typed_receiver.is_some_and(|receiver| self.type_index.standard_integer_available(receiver)) {
            "<rust-integer-byte-conversion>".to_string()
        } else if matches!(inferred_receiver, Some(RustValueType::Sequence(_)))
            && node.method == "last_mut" && node.args.is_empty() {
            "<rust-sequence-last-mut>".to_string()
        } else if matches!(inferred_receiver, Some(RustValueType::Sequence(_)))
            && node.method == "retain" && node.args.len() == 1 && node.turbofish.is_none() {
            // Identify the standard operation, not an unrelated same-leaf helper.
            // Its predicate is still traversed with the proven element type.
            "<rust-sequence-retain>".to_string()
        } else if matches!(inferred_receiver, Some(RustValueType::Iterator(_)))
            && node.method == "max_by_key" && node.args.len() == 1
            && matches!(&node.args[0], Expr::Closure(closure) if closure.inputs.len() == 1) {
            "<rust-iterator-max-by-key>".to_string()
        } else if let Some(receiver_type) = typed_receiver {
            format!("{receiver_type}::{}", node.method)
        } else if receiver.is_empty() {
            node.method.to_string()
        } else {
            format!("{receiver}.{}", node.method)
        };
        if external_marker.is_none() && typed_receiver.is_none()
            && rust_expr_root_ident(&node.receiver)
                .is_some_and(|root| self.parameter_types.contains_key(&root))
            && !known_pure_call(&call)
        {
            self.calls.insert("<dynamic-call>".to_string());
            visit::visit_expr_method_call(self, node);
            return;
        }
        if external_marker.is_none() && rust_expr_root_ident(&node.receiver)
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
        {
            for (position, argument) in node.args.iter().enumerate() {
                let bounded_new_callback = matches!((node.method.to_string().as_str(), position),
                    ("map_or_else", 0 | 1) | ("flat_map", 0) | ("fold", 1))
                    || (node.method == "find_map" && position == 0
                        && matches!(inferred_receiver, Some(RustValueType::Iterator(_))))
                    || (node.method == "retain" && position == 0
                        && matches!(inferred_receiver, Some(RustValueType::Sequence(_))))
                    || (node.method == "and_then" && position == 0
                        && matches!(inferred_receiver, Some(RustValueType::ResultOk(_) | RustValueType::ResultKnown(_, _))))
                    || inferred_receiver.as_ref().is_some_and(|receiver|
                        matches!(receiver, RustValueType::External(_) | RustValueType::ResultKnown(_, _))
                        && receiver.callback_inputs(&node.method.to_string(), position).is_some());
                let typed_predicate = matches!(node.method.to_string().as_str(), "is_some_and" | "any" | "all")
                    && matches!(inferred_receiver, Some(RustValueType::Optional(_) | RustValueType::Iterator(_)));
                if !bounded_new_callback && !typed_predicate && !matches!(node.method.to_string().as_str(), "map" | "filter_map") { continue; }
                let Expr::Path(path) = argument else {
                    if bounded_new_callback && !matches!(argument, Expr::Closure(_)) {
                        self.calls.insert("<dynamic-call>".to_string());
                    }
                    continue;
                };
                if matches!(inferred_receiver, Some(RustValueType::External(_))) && node.method == "walk" && position == 1 {
                    self.calls.insert("<dynamic-call>".into());
                }
                let callable = path
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                if self.dynamic_symbols.contains(symbol_name(&callable)) {
                    self.calls.insert("<dynamic-call>".to_string());
                } else if callable == "char::is_control" && position == 0
                    && matches!(node.method.to_string().as_str(), "any" | "all")
                    && node.args.len() == 1 && node.turbofish.is_none()
                    && matches!(&inferred_receiver, Some(RustValueType::Iterator(element))
                        if **element == RustValueType::Character)
                    && path.qself.is_none() && path.path.leading_colon.is_none()
                    && path.path.segments.iter().all(|part| matches!(part.arguments, syn::PathArguments::None))
                    && self.type_index.unshadowed_external_root("char") {
                    // Close only the standard primitive predicate on characters.
                    // Same-name imports, custom receivers and open callbacks stay checked.
                    self.calls.insert("<rust-character-predicate>".into());
                } else {
                    self.calls.insert(if self.standard_str && callable == "str::to_ascii_uppercase" {
                        "<rust-str-ascii-uppercase>".to_string()
                    } else { callable });
                }
            }
        }
        if let Some(receiver) = inferred_receiver.as_ref() {
            self.visit_expr(&node.receiver);
            for (position, argument) in node.args.iter().enumerate() {
                if let Expr::Closure(closure) = argument {
                    if let Some(inputs) = receiver.callback_inputs(&node.method.to_string(), position)
                        .filter(|inputs| inputs.len() == closure.inputs.len()) {
                        let prior_values = self.value_types.clone();
                        let prior_types = self.parameter_types.clone();
                        let prior_dynamic = self.dynamic_symbols.clone();
                        let prior_callbacks = self.callback_bindings.clone();
                        let mut names = BTreeSet::new();
                        for pattern in &closure.inputs { collect_rust_pattern_identifiers(pattern, &mut names); }
                        for name in &names {
                            self.value_types.remove(name);
                            self.parameter_types.remove(name);
                            self.callback_bindings.remove(name);
                            self.dynamic_symbols.insert(name.clone());
                        }
                        for (pattern, value) in closure.inputs.iter().zip(inputs) {
                            for (name, value) in self.type_index.pattern_bindings(pattern, &value) {
                                if let Some(type_name) = value.name() {
                                    self.parameter_types.insert(name.clone(), type_name.clone());
                                    self.dynamic_symbols.remove(&name);
                                }
                                self.value_types.insert(name, value);
                            }
                        }
                        let prior_return_error = self.return_error.take();
                        if !self.type_index.external_callback_closed(receiver, &node.method.to_string(), position, closure, &self.value_types) {
                            self.calls.insert("<dynamic-call>".into());
                        }
                        self.visit_expr(&closure.body);
                        self.return_error = prior_return_error;
                        self.value_types = prior_values;
                        self.parameter_types = prior_types;
                        self.dynamic_symbols = prior_dynamic;
                        self.callback_bindings = prior_callbacks;
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
        let prior_return_error = self.return_error.take();
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
        self.return_error = prior_return_error;
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
        let inferred = self.type_index.sequence_annotation(&node.pat).or_else(|| node.init.as_ref().and_then(|init| {
            self.type_index
                .expression_type(&init.expr, &self.value_types)
        })).or_else(|| {
            let Pat::Ident(name) = &node.pat else { return None; };
            node.init.as_ref().filter(|init| self.type_index.is_standard_empty_vec(&init.expr)
                || self.type_index.is_standard_empty_map(&init.expr))
                .and_then(|_| self.sequence_constraints.get(&name.ident.to_string()).cloned())
        });
        let mut bound_names = BTreeSet::new();
        collect_rust_pattern_identifiers(&node.pat, &mut bound_names);
        // Inspect the initializer in the old scope, then install the new binding.
        let yaml_value = matches!(&node.pat, Pat::Type(pattern) if exact_yaml_value_type(&pattern.ty));
        let handled = yaml_value && node.init.as_ref()
            .is_some_and(|init| self.visit_yaml_value_initializer(&init.expr));
        if !handled { visit::visit_local(self, node); }
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
        if let Some(value) = inferred {
            for (name, value) in self.type_index.pattern_bindings(&node.pat, &value) {
                if let Some(type_name) = value.name() {
                    self.parameter_types.insert(name.clone(), type_name.clone());
                    self.dynamic_symbols.remove(&name);
                } else if value == RustValueType::Unknown {
                    self.dynamic_symbols.insert(name.clone());
                }
                self.value_types.insert(name, value);
            }
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

fn exact_yaml_value_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.qself.is_none()
        && path.path.segments.len() == 2
        && path.path.segments[0].ident == "serde_yaml_ng"
        && path.path.segments[1].ident == "Value"
        && path.path.segments.iter().all(|segment| matches!(segment.arguments, syn::PathArguments::None)))
}

impl RustCallCollector {
    fn is_yaml_value_parse(&self, call: &ExprCall, contextual_value: bool) -> bool {
        let Expr::Path(path) = call.func.as_ref() else { return false; };
        if path.qself.is_some() || path.path.segments.len() != 2 || call.args.len() != 1
            || path.path.segments[0].ident != "serde_yaml_ng"
            || path.path.segments[1].ident != "from_str"
            || !matches!(path.path.segments[0].arguments, syn::PathArguments::None)
            || self.value_types.contains_key("serde_yaml_ng")
            || !self.type_index.unshadowed_external_root("serde_yaml_ng") { return false; }
        match &path.path.segments[1].arguments {
            syn::PathArguments::None => contextual_value,
            syn::PathArguments::AngleBracketed(arguments) if arguments.args.len() == 1 =>
                matches!(&arguments.args[0], syn::GenericArgument::Type(ty) if exact_yaml_value_type(ty)),
            _ => false,
        }
    }

    // Only wrappers that preserve Result's success type carry the annotation
    // inward. A map callback, arbitrary helper, or custom Deserialize target does not.
    fn visit_yaml_value_initializer(&mut self, expression: &Expr) -> bool {
        match expression {
            Expr::Try(value) => self.visit_yaml_value_initializer(&value.expr),
            Expr::Paren(value) => self.visit_yaml_value_initializer(&value.expr),
            Expr::MethodCall(call) if call.method == "map_err" && call.args.len() == 1 => {
                if !self.visit_yaml_value_initializer(&call.receiver) { return false; }
                self.visit_expr(&call.args[0]);
                true
            }
            Expr::Call(call) if self.is_yaml_value_parse(call, true) => {
                self.calls.insert("<rust-yaml-value-from-str>".into());
                visit::visit_expr_call(self, call);
                true
            }
            _ => false,
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
    inherent_self: Option<RustValueType>,
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
            return_error: self.type_index.return_error(&node.sig),
            type_index: self.type_index.with_generics(&node.sig.generics),
            standard_str: self.type_index.standard_str_available(&node.sig, &node.block),
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
            return_error: self.type_index.return_error(&node.sig),
            type_index: self.type_index.with_generics(&node.sig.generics),
            standard_str: self.type_index.standard_str_available(&node.sig, &node.block),
            value_types: self.type_index.parameters(&node.sig),
            dynamic_symbols: rust_dynamic_parameters(node.sig.inputs.iter()),
            parameter_types: rust_parameter_types(node.sig.inputs.iter()),
            ..RustCallCollector::default()
        };
        if node.sig.receiver().is_some() {
            if let Some(value) = &self.inherent_self {
                calls.value_types.insert("self".into(), value.clone());
            } else {
                calls.dynamic_symbols.insert("self".into());
            }
        }
        if node.sig.unsafety.is_some() {
            calls.authorities.insert("unsafe".to_string());
        }
        calls.visit_block(&node.block);
        if let Some(RustValueType::Named(owner)) = &self.inherent_self {
            // Self names this exact source-owned inherent impl, never a
            // same-leaf helper or a same-named type in another source file.
            let resolve_self = |call: &String| call.strip_prefix("Self::")
                .map(|method| format!("{owner}::{method}"))
                .unwrap_or_else(|| call.clone());
            calls.calls = calls.calls.iter().map(resolve_self).collect();
            calls.unsafe_calls = calls.unsafe_calls.iter().map(resolve_self).collect();
        }
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
        let prior_index = self.type_index.clone();
        let prior_self = self.inherent_self.take();
        self.type_index = self.type_index.with_generics(&node.generics);
        if node.trait_.is_none() && self.owner.is_empty() {
            self.inherent_self = self.type_index.inherent_self_type(&node.self_ty);
        }
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
        self.type_index = prior_index;
        self.inherent_self = prior_self;
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
        inherent_self: None,
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
    aliases.retain(|name, target| safe_aliases.contains(name)
        && (!target.starts_with("std::") || types.for_path(path).unshadowed_external_root("std")));
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
    if let Some((owner_reference, method)) = reference.rsplit_once("::") {
        if let Some(owner) = rust_exact_item_reference(path, owner_reference, sources, depth) {
            let (owner_path, owner_name) = owner.split_once('#')?;
            let file = syn::parse_file(sources.get(owner_path)?).ok()?;
            let methods = file.items.iter().filter_map(|item| match item {
                syn::Item::Impl(item) if item.trait_.is_none() && matches!(item.self_ty.as_ref(), Type::Path(owner) if owner.path.get_ident().is_some_and(|name| name == owner_name)) => Some(item),
                _ => None,
            }).flat_map(|item| item.items.iter()).filter(|item| matches!(item, syn::ImplItem::Fn(function) if function.sig.ident == method)).count();
            if methods == 1 {
                return Some(format!("{owner_path}#{owner_name}::{method}"));
            }
        }
    }
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
            let file = syn::parse_file(sources.get(path)?).ok()?;
            if file.items.iter().any(|item| match item {
                syn::Item::Mod(item) => item.ident == parts[0],
                syn::Item::Struct(item) => item.ident == parts[0],
                syn::Item::Enum(item) => item.ident == parts[0],
                syn::Item::Type(item) => item.ident == parts[0],
                syn::Item::ExternCrate(item) => item.rename.as_ref().map(|(_, name)| name).unwrap_or(&item.ident) == parts[0],
                _ => false,
            }) { return None; }
            let caller = path.split_once("/src/").map(|(prefix, _)| prefix).unwrap_or("");
            sources
                .get(&format!("rms-metadata/rust-crate-alias/{caller}/{}", parts[0]))
                .or_else(|| sources.get(&format!("rms-metadata/rust-crate-alias/{}", parts[0])))?
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
                if function.sig.ident == reference
                    && function.attrs.iter().all(|attribute| attribute.path().is_ident("doc")) =>
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
            let module_path = if module == "crate" {
                "lib".to_string()
            } else {
                module.strip_prefix("crate::")?.replace("::", "/")
            };
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
            if name.ident == "self" {
                if let Some(local) = prefix.last() { aliases.insert(local.clone(), prefix.join("::")); }
                return;
            }
            let mut target = prefix;
            target.push(name.ident.to_string());
            aliases.insert(name.ident.to_string(), target.join("::"));
        }
        UseTree::Rename(rename) => {
            let mut target = prefix;
            if rename.ident != "self" { target.push(rename.ident.to_string()); }
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
    checked_continuation: bool,
    string_split: bool,
    foundation_values: bool,
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
                if swift_global_names.string_split {
                    names.extend(swift_string_split_collection_names(
                        tree.root_node(),
                        node,
                        source,
                    ));
                }
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
            if binding == "swift" {
                if swift_global_names.foundation_values {
                    swift_refine_guard_values(node, source, &mut calls);
                }
                for parameter in swift_direct_callable_parameters(node, source) {
                    let dynamic = calls.iter().filter(|call| {
                        *call == &parameter || call.strip_prefix(&parameter).is_some_and(|tail| {
                            tail.starts_with('(') && matching_delimiter(tail, 0, '(', ')') == Some(tail.len() - 1)
                        })
                    }).cloned().collect::<Vec<_>>();
                    if !dynamic.is_empty() {
                        for call in dynamic { calls.remove(&call); }
                        calls.insert("<dynamic-call>".to_string());
                    }
                }
            }
            if binding == "swift" && swift_global_names.checked_continuation {
                // Only the standard control primitive is closed. Calls inside
                // its literal closure were collected and remain in the graph.
                let mut invocations = Vec::new();
                collect_nodes_of_kind(node, "call_expression", &mut invocations);
                for intrinsic in ["withCheckedContinuation", "Swift.withCheckedContinuation"] {
                    let matching = invocations
                        .iter()
                        .filter(|call| {
                            call.child_by_field_name("function")
                                .or_else(|| call.child_by_field_name("name"))
                                .or_else(|| call.named_child(0))
                                .and_then(|callee| callee.utf8_text(source.as_bytes()).ok())
                                .map(normalize_call)
                                .as_deref()
                                == Some(intrinsic)
                        })
                        .collect::<Vec<_>>();
                    if !matching.is_empty()
                        && matching.iter().all(|call| {
                            call.utf8_text(source.as_bytes())
                                .ok()
                                .and_then(|text| text.strip_prefix(intrinsic))
                                .is_some_and(|tail| tail.trim_start().starts_with('{'))
                        })
                    {
                        calls.remove(intrinsic);
                    }
                }
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
    let reassigned = shadowed.clone();
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
        for (module, member, alias) in python_import_bindings(line) {
            if statement.parent() != Some(root) {
                shadowed.insert(alias);
                continue;
            }
            imports.entry(alias).or_default().push((module, member));
        }
    }
    let mut recognized = BTreeMap::new();
    for (alias, imported) in imports {
        if imported.len() != 1 || shadowed.contains(&alias) {
            continue;
        }
        let (module, member) = &imported[0];
        if !matches!(
            module.as_str(),
            "time" | "shutil" | "pathlib" | "urllib.request"
        ) {
            continue;
        }
        let root_module = module.split('.').next().unwrap_or(module);
        if sources.keys().any(|path| {
            path.ends_with(&format!("/{module}.py"))
                || path == &format!("{module}.py")
                || path.ends_with(&format!("/{module}/__init__.py"))
                || path.ends_with(&format!("/{root_module}.py"))
                || path == &format!("{root_module}.py")
                || path.contains(&format!("/{root_module}/"))
                || path.starts_with(&format!("{root_module}/"))
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
        if let Some(parameters) = definition.child_by_field_name("parameters") {
            let mut cursor = parameters.walk();
            for parameter in parameters.named_children(&mut cursor) {
                let (Some(name), Some(value)) = (
                    parameter.child_by_field_name("name"),
                    parameter.child_by_field_name("value"),
                ) else {
                    continue;
                };
                if name.kind() != "identifier" || value.kind() != "identifier" {
                    continue;
                }
                let (Ok(name), Ok(default)) = (
                    name.utf8_text(source.as_bytes()),
                    value.utf8_text(source.as_bytes()),
                ) else {
                    continue;
                };
                if !function.calls.contains(name) || reassigned.contains(default) {
                    continue;
                }
                let defaults = definitions
                    .iter()
                    .filter(|candidate| {
                        candidate.parent() == Some(root)
                            && candidate.start_byte() < definition.start_byte()
                            && function_node_name(**candidate, source).as_deref() == Some(default)
                    })
                    .collect::<Vec<_>>();
                if defaults.len() == 1 {
                    // The known default adds a reachable path. The supplied
                    // callable retains dynamic authority; this is not specialization.
                    function.calls.insert(format!("{path}#{default}"));
                }
            }
        }
        // A direct parameter invocation has a known dispatch mechanism, not
        // a known implementation. Preserve its dynamic authority. Require all
        // occurrences to be the declaration or direct calls so rebinding,
        // nested shadowing, aliases, and captured values remain unresolved.
        let mut identifiers = Vec::new();
        collect_nodes_of_kind(definition, "identifier", &mut identifiers);
        let mut invocations = Vec::new();
        collect_nodes_of_kind(definition, "call", &mut invocations);
        let direct_count = |name: &str| invocations.iter().filter(|call| {
            call.child_by_field_name("function").is_some_and(|callee|
                callee.kind() == "identifier" && callee.utf8_text(source.as_bytes()).ok() == Some(name))
        }).count();
        if let Some(parameters) = definition.child_by_field_name("parameters") {
            let mut cursor = parameters.walk();
            for parameter in parameters.named_children(&mut cursor) {
                let name = if parameter.kind() == "identifier" { Some(parameter) }
                    else { parameter.child_by_field_name("name").or_else(|| parameter.named_child(0)) };
                let Some(name) = name.filter(|node| node.kind() == "identifier")
                    .and_then(|node| node.utf8_text(source.as_bytes()).ok()) else { continue; };
                let count = direct_count(name);
                let occurrences = identifiers.iter().filter(|node|
                    node.utf8_text(source.as_bytes()).ok() == Some(name)).count();
                if count > 0 && occurrences == count + 1 && function.calls.remove(name) {
                    function.calls.insert("<dynamic-call>".to_string());
                }
            }
        }
        // Reflection can invoke user-defined attribute/metaclass behavior.
        // It is never a purity exemption. Shadowed names stay unresolved.
        for builtin in ["getattr", "type"] {
            if !shadowed.contains(builtin) && direct_count(builtin) > 0
                && function.calls.remove(builtin) {
                function.calls.insert("<dynamic-call>".to_string());
            }
        }
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
                        ("urlopen", "python-stdlib.urllib.request.urlopen"),
                        ("Request", "python-stdlib.urllib.request.Request"),
                    ] {
                        if !matches!(
                            (module.as_str(), operation),
                            ("time", "sleep")
                                | ("shutil", "copy2")
                                | ("pathlib", "Path")
                                | ("urllib.request", "urlopen" | "Request")
                        ) {
                            continue;
                        }
                        let expected = if member.is_empty() {
                            format!("{alias}.{operation}")
                        } else {
                            if member != operation {
                                continue;
                            }
                            format!(
                                "{}.py#{member}",
                                normalized_import_path(path, &module.replace('.', "/"))
                            )
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

fn python_import_bindings(statement: &str) -> Vec<(String, String, String)> {
    let (module, members) = if let Some(rest) = statement.strip_prefix("from ") {
        let Some((module, members)) = rest.split_once(" import ") else {
            return Vec::new();
        };
        (
            Some(module.trim()),
            members.trim().trim_start_matches('(').trim_end_matches(')'),
        )
    } else if let Some(rest) = statement.strip_prefix("import ") {
        (None, rest)
    } else {
        return Vec::new();
    };
    members
        .split(',')
        .filter_map(|member| {
            let words = member.split_whitespace().collect::<Vec<_>>();
            let (name, alias) = match words.as_slice() {
                [name] => (*name, *name),
                [name, "as", alias] => (*name, *alias),
                _ => return None,
            };
            if !name.split('.').all(is_simple_identifier)
                || !alias.split('.').all(is_simple_identifier)
            {
                return None;
            }
            Some((
                module.unwrap_or(name).to_string(),
                if module.is_some() {
                    name.to_string()
                } else {
                    String::new()
                },
                alias.to_string(),
            ))
        })
        .collect()
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
    let mut shadowed = BTreeSet::new();
    let mut declared_methods = BTreeSet::new();
    for source in sources.values() {
        let mut parser = Parser::new();
        if parser.set_language(&language).is_err() {
            continue;
        }
        let Some(tree) = parser.parse(source, None) else {
            continue;
        };
        let mut identifiers = Vec::new();
        collect_nodes_of_kind(tree.root_node(), "simple_identifier", &mut identifiers);
        for identifier in identifiers {
            if identifier.utf8_text(source.as_bytes()).ok() == Some("UUID")
                && !identifier.parent().is_some_and(|parent| parent.kind() == "call_expression"
                    && parent.named_child(0) == Some(identifier)) {
                shadowed.insert("UUID".to_string());
            }
        }
        for kind in [
            "function_declaration",
            "property_declaration",
            "parameter",
            "lambda_parameter",
            "class_declaration",
            "typealias_declaration",
        ] {
            let mut declarations = Vec::new();
            collect_nodes_of_kind(tree.root_node(), kind, &mut declarations);
            for declaration in declarations {
                if let Some(name) = declaration
                    .child_by_field_name("name")
                    .and_then(|name| name.utf8_text(source.as_bytes()).ok()
                        .filter(|name| is_simple_identifier(name)).map(str::to_string)
                        .or_else(|| first_simple_identifier(name, source)))
                {
                    if kind == "function_declaration" { declared_methods.insert(name.clone()); }
                    shadowed.insert(name);
                }
            }
        }
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
        foundation_values: !["UUID", "String", "Dictionary", "Foundation", "Swift", "lowercased", "uuidString", "trimmingCharacters"]
            .iter().any(|name| shadowed.contains(*name))
            && !declared_methods.contains("data"),
        checked_continuation: !shadowed.contains("withCheckedContinuation")
            && !shadowed.contains("Swift"),
        string_split: !["String", "Swift", "split", "map"]
            .iter()
            .any(|name| shadowed.contains(*name)),
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

pub(crate) fn function_node_name(node: Node<'_>, source: &str) -> Option<String> {
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

pub(crate) fn tree_sitter_qualified_name(node: Node<'_>, source: &str, name: &str) -> String {
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

// Bounded Foundation value flow. Only exact guarded casts/constructors and
// immutable String normalization qualify. Every name use must be a member or
// subscript receiver after its declaration; shadowing and aliases stay open.
fn swift_refine_guard_values(node: Node<'_>, source: &str, calls: &mut BTreeSet<String>) {
    if node.has_error() || !source.lines().any(|line| line.trim() == "import Foundation") { return; }
    let compact = |node: Node<'_>| node.utf8_text(source.as_bytes()).unwrap_or_default()
        .chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let mut identifiers = Vec::new();
    collect_nodes_of_kind(node, "simple_identifier", &mut identifiers);
    let stable = |name: &str, declaration: Node<'_>, available: usize| {
        identifiers.iter().filter(|id| id.utf8_text(source.as_bytes()).ok() == Some(name)).all(|id| {
            if id.start_byte() >= declaration.start_byte() && id.end_byte() <= declaration.end_byte() { return true; }
            let receiver = id.parent().filter(|parent| parent.kind() == "prefix_expression"
                && compact(*parent) == format!("!{name}")).unwrap_or(*id);
            id.start_byte() >= available && receiver.parent().is_some_and(|parent| {
                (parent.kind() == "navigation_expression" || parent.kind() == "call_expression")
                    && parent.named_child(0) == Some(receiver)
                    && source.get(receiver.end_byte()..parent.end_byte()).is_some_and(|tail|
                        tail.trim_start().starts_with('.') || tail.trim_start().starts_with('['))
            })
        })
    };
    let mut strings = BTreeSet::new();
    let mut parameters = Vec::new();
    collect_nodes_of_kind(node, "parameter", &mut parameters);
    for parameter in parameters {
        let Some(name) = parameter.child_by_field_name("name") else { continue; };
        let name_text = compact(name);
        if compact(parameter).ends_with(":String") && stable(&name_text, name, parameter.end_byte()) {
            strings.insert(name_text);
        }
    }
    let mut properties = Vec::new();
    collect_nodes_of_kind(node, "property_declaration", &mut properties);
    for property in properties {
        let (Some(name), Some(value)) = (property.child_by_field_name("name"), property.child_by_field_name("value")) else { continue; };
        let name_text = compact(name);
        if !property.utf8_text(source.as_bytes()).unwrap_or_default().trim_start().starts_with("let ")
            || !stable(&name_text, name, value.end_byte()) { continue; }
        if compact(value).strip_suffix(".trimmingCharacters(in:.whitespacesAndNewlines)")
            .is_some_and(|receiver| strings.contains(receiver)) {
            strings.insert(name_text);
        }
    }
    let mut guards = Vec::new();
    collect_nodes_of_kind(node, "guard_statement", &mut guards);
    for guard in guards {
        for i in 0..guard.child_count() {
            if guard.field_name_for_child(i as u32) != Some("bound_identifier") { continue; }
            let Some(name) = guard.child(i) else { continue; };
            let Some(value) = name.next_named_sibling().filter(|rhs|
                source.get(name.end_byte()..rhs.start_byte()).is_some_and(|text| text.trim() == "=")) else { continue; };
            let name_text = compact(name);
            if !is_simple_identifier(&name_text) || !stable(&name_text, name, value.end_byte()) { continue; }
            let expression = compact(value);
            if value.kind() == "as_expression" && expression.ends_with("as?[String:Any]") {
                calls.remove(&name_text);
            }
            if expression.strip_prefix("UUID(uuidString:").is_some_and(|tail|
                tail.ends_with(')') && is_simple_identifier(&tail[..tail.len()-1])) {
                calls.remove(&format!("{name_text}.uuidString.lowercased"));
            }
        }
    }
    let mut invocations = Vec::new();
    collect_nodes_of_kind(node, "call_expression", &mut invocations);
    for receiver in strings {
        let call_name = format!("{receiver}.data");
        let matching = invocations.iter().filter(|call| call.named_child(0).is_some_and(|callee| compact(callee) == call_name)).collect::<Vec<_>>();
        if !matching.is_empty() && matching.iter().all(|call| compact(**call) == format!("{receiver}.data(using:.utf8)")) {
            calls.remove(&call_name);
        }
    }
}

fn swift_direct_callable_parameters(node: Node<'_>, source: &str) -> BTreeSet<String> {
    let Some(declaration) = node.utf8_text(source.as_bytes()).ok() else { return BTreeSet::new(); };
    let Some(start) = declaration.find('(') else { return BTreeSet::new(); };
    let Some(end) = matching_delimiter(declaration, start, '(', ')') else { return BTreeSet::new(); };
    let parameters = split_top_level(&declaration[start + 1..end], ',').into_iter().filter_map(|parameter| {
        let colon = top_level_delimiter(parameter, ':')?;
        let name = parameter[..colon].split_whitespace().next_back()?;
        (is_simple_identifier(name) && name != "_").then(|| name.to_string())
    }).collect::<BTreeSet<_>>();
    let mut candidates = parameters.iter().map(|name| (name.clone(), 1usize)).collect::<BTreeMap<_, _>>();
    let mut owner = node.parent();
    while let Some(parent) = owner {
        if matches!(parent.kind(), "class_declaration" | "struct_declaration" | "actor_declaration") {
            let mut fields = Vec::new();
            collect_nodes_of_kind(parent, "property_declaration", &mut fields);
            for field in fields {
                if nearest_function_ancestor(field).is_some() || !node_has_kind(field, "function_type") { continue; }
                let mut field_owner = field.parent();
                while field_owner.is_some_and(|owner| !matches!(owner.kind(), "class_declaration" | "struct_declaration" | "actor_declaration")) {
                    field_owner = field_owner.and_then(|owner| owner.parent());
                }
                if field_owner != Some(parent) { continue; }
                let Some(name) = field.child_by_field_name("name").and_then(|name| first_simple_identifier(name, source)) else { continue; };
                if !parameters.contains(&name) { candidates.insert(name, 0); }
            }
            break;
        }
        owner = parent.parent();
    }
    let mut identifiers = Vec::new();
    collect_nodes_of_kind(node, "simple_identifier", &mut identifiers);
    let mut invocations = Vec::new();
    collect_nodes_of_kind(node, "call_expression", &mut invocations);
    candidates.into_iter().filter_map(|(parameter, declaration_count)| {
        let occurrences = identifiers.iter().filter(|identifier| identifier.utf8_text(source.as_bytes()).ok() == Some(parameter.as_str())).count();
        let calls = invocations.iter().filter(|call| {
            call.child_by_field_name("function")
                .or_else(|| call.child_by_field_name("name"))
                .or_else(|| call.named_child(0))
                .is_some_and(|callee| callee.kind() == "simple_identifier"
                    && callee.utf8_text(source.as_bytes()).ok() == Some(parameter.as_str())
                    && source.get(callee.end_byte()..call.end_byte()).is_some_and(|tail| tail.trim_start().starts_with('(')))
        }).count();
        // Admit only the parameter declaration (or enclosing stored callable)
        // plus direct invocations. Any
        // rebinding, capture, nested parameter, alias, or other use is left open.
        // Callable identity implies dynamic dispatch, never callback purity.
        (calls > 0 && occurrences == calls + declaration_count).then_some(parameter)
    }).collect()
}

pub(crate) fn swift_callable_selector(node: Node<'_>, source: &str) -> Option<String> {
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

pub(crate) fn matching_delimiter(source: &str, start: usize, open: char, close: char) -> Option<usize> {
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

pub(crate) fn split_top_level(source: &str, delimiter: char) -> Vec<&str> {
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

fn swift_string_split_collection_names(
    root: Node<'_>,
    function: Node<'_>,
    source: &str,
) -> BTreeSet<String> {
    let mut scopes = vec![function];
    while let Some(parent) = nearest_function_ancestor(*scopes.last().unwrap()) {
        scopes.push(parent);
    }
    let visible =
        |node| nearest_function_ancestor(node).is_some_and(|owner| scopes.contains(&owner));
    let mut parameters = Vec::new();
    collect_nodes_of_kind(root, "parameter", &mut parameters);
    collect_nodes_of_kind(root, "lambda_parameter", &mut parameters);
    let mut names = BTreeMap::<String, usize>::new();
    let mut strings = BTreeSet::new();
    for parameter in parameters
        .into_iter()
        .filter(|parameter| visible(*parameter))
    {
        let Some(name) = parameter
            .child_by_field_name("name")
            .and_then(|name| first_simple_identifier(name, source))
        else {
            continue;
        };
        *names.entry(name.clone()).or_default() += 1;
        let text = parameter.utf8_text(source.as_bytes()).unwrap_or_default();
        if text
            .split_once(':')
            .map(|(_, ty)| ty.trim())
            .is_some_and(|ty| matches!(ty, "String" | "Swift.String"))
        {
            strings.insert(name);
        }
    }
    let mut declarations = Vec::new();
    collect_nodes_of_kind(root, "property_declaration", &mut declarations);
    let declarations = declarations
        .into_iter()
        .filter(|declaration| visible(*declaration))
        .collect::<Vec<_>>();
    for declaration in &declarations {
        if let Some(name) = declaration
            .child_by_field_name("name")
            .and_then(|name| first_simple_identifier(name, source))
        {
            *names.entry(name).or_default() += 1;
        }
    }
    let mut result = BTreeSet::new();
    for declaration in declarations {
        let Some(name) = declaration
            .child_by_field_name("name")
            .and_then(|name| first_simple_identifier(name, source))
        else {
            continue;
        };
        if names.get(&name) != Some(&1)
            || !declaration
                .utf8_text(source.as_bytes())
                .unwrap_or_default()
                .trim_start()
                .starts_with("let ")
        {
            continue;
        }
        let Some(value) = declaration.child_by_field_name("value") else {
            continue;
        };
        let compact = value
            .utf8_text(source.as_bytes())
            .unwrap_or_default()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        let Some((receiver, _)) = compact.split_once(".split(") else {
            continue;
        };
        if strings.contains(receiver)
            && names.get(receiver) == Some(&1)
            && compact.ends_with(").map(String.init)")
        {
            result.insert(name);
        }
    }
    result
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
    fn rust_set_elements_and_standard_text_comparison_keep_exact_effects() {
        let source = r#"use std::collections::BTreeSet;
struct Path { text: String }
impl Path { fn text(&self) -> &str { &self.text } }
struct Policy { paths: BTreeSet<Path> }
impl Policy { fn check(&self) -> bool { self.paths.iter().any(|path| path.text().eq_ignore_ascii_case("system")) } }
fn decide(policy: &Policy) { policy.check(); }"#;
        let result = report("rust", "src/lib.rs", source, expectation("src/lib.rs#decide", "pure", &[]));
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        for changed in [
            source.replace("&self.text }", "std::fs::read(\"x\"); &self.text }"),
            source.replace("use std::collections::BTreeSet;", "use custom::BTreeSet;"),
            source.replace("use std::collections::BTreeSet;", "mod std {} use std::collections::BTreeSet;"),
        ] {
            assert_eq!(report("rust", "src/lib.rs", &changed, expectation("src/lib.rs#decide", "pure", &[])).result, AnalysisResult::Fail);
        }
        let concrete = "fn decide(value: &str) { let value = value.to_owned(); value.split('/').any(|part| part.eq_ignore_ascii_case(\".git\")); }";
        assert_eq!(report("rust", "src/lib.rs", concrete, expectation("src/lib.rs#decide", "pure", &[])).result, AnalysisResult::Pass);
        let generic = concrete.replace("value: &str", "value: impl Into<String>").replace("value.to_owned()", "value.into()");
        assert_eq!(report("rust", "src/lib.rs", &generic, expectation("src/lib.rs#decide", "pure", &[])).result, AnalysisResult::Fail);
    }

    #[test]
    fn rust_match_joins_and_if_let_keep_exact_payload_types_and_scope() {
        let source = r#"struct Row;
impl Row { fn check(&self) {} }
enum Entry { File(Row), Directory(Row) }
enum Request { One(Vec<Entry>), Two(Vec<Entry>) }
fn decide(request: &Request) {
    let entries = match request { Request::One(items) | Request::Two(items) => items };
    for entry in entries { if let Entry::File(row) = entry { row.check(); } }
}"#;
        assert_eq!(report("rust", "src/lib.rs", source, expectation("src/lib.rs#decide", "pure", &[])).result, AnalysisResult::Pass);
        for changed in [
            source.replace("fn check(&self) {}", "fn check(&self) { std::fs::read(\"x\"); }"),
            source.replace("Two(Vec<Entry>)", "Two(Vec<Unknown>)"),
            source.replace("Entry::File(row) = entry", "Other::File(row) = entry"),
            source.replace("request: &Request", "request: &Request, row: &dyn Unknown").replace("row.check(); }", "row.check(); } else { row.check(); }"),
        ] {
            let result = report("rust", "src/lib.rs", &changed, expectation("src/lib.rs#decide", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{changed}\n{result:#?}");
        }
    }

    #[test]
    fn rust_empty_map_insertion_proves_value_type_not_callback_purity() {
        let source = r#"use std::collections::BTreeMap;
struct Row;
impl Row { fn new() -> Self { Self } fn check(&self) -> bool { true } }
fn decide() {
    let mut rows = BTreeMap::new();
    rows.insert(1, Row::new());
    rows.get(&1).is_some_and(|row| row.check());
}"#;
        for valid in [source.to_string(), source.replace("|row| row.check()", "Row::check")] {
            let result = report("rust", "src/lib.rs", &valid, expectation("src/lib.rs#decide", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
            let effectful = valid.replace("{ true }", "{ std::fs::read(\"x\"); true }");
            assert_eq!(report("rust", "src/lib.rs", &effectful, expectation("src/lib.rs#decide", "pure", &[])).result, AnalysisResult::Fail);
        }
        for changed in [
            source.replace("use std::collections::BTreeMap;", "use custom::BTreeMap;"),
            source.replace("rows.get(&1)", "let rows = unknown(); rows.get(&1)"),
            source.replace("rows.insert(1, Row::new());", "rows.insert(1, Row::new()); rows.insert(2, \"other\");"),
        ] {
            assert_eq!(report("rust", "src/lib.rs", &changed, expectation("src/lib.rs#decide", "pure", &[])).result, AnalysisResult::Fail);
        }
    }

    #[test]
    fn rust_split_inclusive_requires_standard_text_and_literal_pattern() {
        for source in [
            "fn decide(raw: &str) { raw.split_inclusive('\\n').next(); }",
            "fn decide(bytes: &[u8]) -> Result<(), ()> { let raw = std::str::from_utf8(bytes).map_err(|_| ())?; for line in raw[1..].split_inclusive(\"\\n\") { line.len(); } Ok(()) }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("src/lib.rs#decide", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        }
        for source in [
            "fn decide(raw: &dyn Custom) { raw.split_inclusive('\\n'); }",
            "fn decide(raw: &str, callback: impl FnMut(char) -> bool) { raw.split_inclusive(callback); }",
            "fn decide(raw: &str) { raw.split_inclusive(|_| { std::fs::read(\"x\"); true }); }",
            "struct str; impl str { fn split_inclusive(&self, _: char) { std::fs::read(\"x\"); } } fn decide(raw: &str) { raw.split_inclusive('\\n'); }",
            "mod std { pub mod str { pub fn from_utf8(_: &[u8]) -> Result<Other, ()> { todo!() } } } fn decide(bytes: &[u8]) -> Result<(), ()> { let raw = std::str::from_utf8(bytes)?; raw.split_inclusive('\\n'); Ok(()) }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("src/lib.rs#decide", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}\n{result:#?}");
        }
    }

    #[test]
    fn rust_yaml_ng_parse_is_bounded_to_library_value() {
        for source in [
            "fn decide(raw: &str) { let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(raw).map_err(|e| e.to_string())?; }",
            "fn decide(raw: &str) { serde_yaml_ng::from_str::<serde_yaml_ng::Value>(raw); }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("src/lib.rs#decide", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        }
        for source in [
            "fn decide(raw: &str) { serde_yaml_ng::from_str::<Custom>(raw); }",
            "fn decide(raw: &str) { serde_yaml_ng::from_str(raw); }",
            "fn decide(raw: &str) { let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str::<Custom>(raw).map(|v| v.into())?; }",
            "fn decide(raw: &str) { let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_reader(raw)?; }",
            "fn decide(raw: &str) { let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(raw).map_err(|e| { std::fs::read(\"x\"); e })?; }",
            "mod serde_yaml_ng { pub struct Value; pub fn from_str(_: &str) -> Value { std::fs::read(\"x\"); Value } } fn decide(raw: &str) { let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(raw); }",
            "use arbitrary as serde_yaml_ng; fn decide(raw: &str) { let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(raw)?; }",
            "extern crate arbitrary as serde_yaml_ng; fn decide(raw: &str) { serde_yaml_ng::from_str::<serde_yaml_ng::Value>(raw); }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("src/lib.rs#decide", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}\n{result:#?}");
        }
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
    fn rust_self_calls_keep_enclosing_source_and_impl_identity() {
        let sources = BTreeMap::from([
            ("src/lib.rs".into(), "struct Owner; impl Owner { fn run() { Self::validated(); } fn validated() {} } struct Other; impl Other { fn validated() { std::fs::read(\"x\"); } }".into()),
            ("src/other.rs".into(), "struct Owner; impl Owner { fn validated() { std::fs::read(\"x\"); } }".into()),
        ]);
        let check = |sources| analyze(AnalysisInput {
            binding: "rust".into(), source_digest: "test".into(), tool_digest: "test".into(), sources,
            semantic_functions: vec![expectation("src/lib.rs#Owner::run", "pure", &[])],
            authority_facades: Vec::new(), trusted_external_calls: BTreeSet::new(),
        });
        let result = check(sources.clone());
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].direct_calls.contains(&"src/lib.rs#Owner::validated".into()));
        let mut missing = sources.clone();
        missing.get_mut("src/lib.rs").unwrap().replace_range(..,
            "struct Owner; impl Owner { fn run() { Self::validated(); } } struct Other; impl Other { fn validated() {} }");
        assert_eq!(check(missing).result, AnalysisResult::Fail);
        let mut effectful = sources;
        *effectful.get_mut("src/lib.rs").unwrap() = "struct Owner; impl Owner { fn run() { Self::validated(); } fn validated() { std::fs::read(\"x\"); } }".into();
        let result = check(effectful);
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.contains(&"filesystem".into()));
    }

    #[test]
    fn rust_qualified_receiver_never_resolves_an_unrelated_leaf() {
        for source in [
            "struct Local; impl Local { fn epoch(&self) -> u64 { 1 } } fn run(s: &MissingScope) -> u64 { s.epoch() }",
            "fn epoch() -> u64 { 1 } fn run(s: &MissingScope) -> u64 { s.epoch() }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
            assert!(!result.functions[0].resolved_callees.contains(&"Local::epoch".into()));
            assert!(!result.functions[0].resolved_callees.contains(&"epoch".into()));
        }
    }

    #[test]
    fn rust_root_glob_and_documented_provider_keep_exact_crate_identity() {
        let sources = BTreeMap::from([
            ("src/lib.rs".into(), "mod transition; pub use crate::transition::transition_record;".into()),
            ("src/driver.rs".into(), "use crate::*; fn run() { transition_record(); } fn external() { provider::invoke(); }".into()),
            ("src/transition.rs".into(), "/// Pure local record.\npub fn transition_record() {}".into()),
            ("dependencies/provider/src/lib.rs".into(), "/// Effectful facade.\npub fn invoke() { transition_record(); } fn transition_record() { std::fs::read_to_string(\"file\"); }".into()),
            ("rms-metadata/rust-crate-alias/provider".into(), "dependencies/provider".into()),
        ]);
        let result = analyze(AnalysisInput {
            binding: "rust".into(), source_digest: "test".into(), tool_digest: "test".into(), sources,
            semantic_functions: vec![expectation("src/driver.rs#run", "pure", &[]), expectation("src/driver.rs#external", "effectful", &["filesystem"])],
            authority_facades: Vec::new(), trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn rust_grouped_self_import_preserves_stdio_and_rejects_shadows() {
        for import in ["use std::io::{self, Read};", "use std::io::{self as input, Read};"] {
            let name = if import.contains("as input") { "input" } else { "io" };
            let source = format!("{import} fn run() {{ {name}::stdin(); }}");
            let result = report("rust", "src/lib.rs", &source, expectation("run", "effectful", &["process"]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        }
        for source in [
            "#[cfg(feature=\"maybe\")] use std::io::{self, Read}; fn run() { io::stdin(); }",
            "use std::io::{self, Read}; fn run<io>() { io::stdin(); }",
            "use std::io::{self, Read}; fn run() { use other as io; io::stdin(); }",
            "use std::io::{self, Read}; fn run() { use other::*; io::stdin(); }",
            "mod std {} use std::io::{self, Read}; fn run() { io::stdin(); }",
        ] {
            assert_eq!(report("rust", "src/lib.rs", source, expectation("run", "effectful", &["process"])).result, AnalysisResult::Fail, "{source}");
        }
        let constructors = "enum Input { Ready } fn run() { use Input::*; let value = Some(Ready); let values: Vec<Input> = Vec::new(); }";
        assert_eq!(report("rust", "src/lib.rs", constructors, expectation("run", "pure", &[])).result, AnalysisResult::Pass);
    }

    #[test]
    fn rust_local_enum_alias_constructors_preserve_purity_without_blessing_calls() {
        let definitions = "enum State { Pending(u8) } impl State { fn Make() { std::env::var(\"X\"); } }";
        for body in [
            "use State as S; S::Pending(1);",
            "use State as S; { S::Pending(1); }",
        ] {
            let source = format!("{definitions} fn run() {{ {body} }}");
            let result = report("rust", "src/lib.rs", &source, expectation("run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        }
        for (signature, body) in [
            ("", "use unknown::State as S; S::Pending(1);"),
            ("", "#[cfg(feature=\"maybe\")] use State as S; S::Pending(1);"),
            ("", "use State as S; use Other as S; S::Pending(1);"),
            ("", "use State as S; { struct S; S::Pending(1); }"),
            ("<State>", "use State as S; S::Pending(1);"),
            ("", "use State as S; S::Make();"),
            ("", "use State as S; S::Missing(1);"),
            ("", "use State as S; S::Pending();"),
            ("", "use State as S; S::Pending({ std::env::var(\"X\"); 1 });"),
            ("", "use State as S; S::Pending(unknown());"),
        ] {
            let source = format!("{definitions} fn run{signature}() {{ {body} }}");
            assert_eq!(report("rust", "src/lib.rs", &source, expectation("run", "pure", &[])).result, AnalysisResult::Fail, "{source}");
        }
        let result = analyze(AnalysisInput { binding: "rust".into(), source_digest: "test".into(), tool_digest: "test".into(),
            sources: BTreeMap::from([
                ("src/lib.rs".into(), "mod representation; pub use crate::representation::*;".into()),
                ("src/representation.rs".into(), "pub enum State { Pending(u8) }".into()),
                ("src/transition.rs".into(), "use crate::*; fn run() { use State as S; S::Pending(1); }".into()),
            ]),
            semantic_functions: vec![expectation("src/transition.rs#run", "pure", &[])],
            authority_facades: vec![], trusted_external_calls: BTreeSet::new() });
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn rust_nested_cli_resolves_only_its_exact_owning_library() {
        let sources = BTreeMap::from([
            ("src/lib.rs".into(), "mod repo; pub use crate::repo::Repo;".into()),
            ("src/repo.rs".into(), "pub struct Repo; impl Repo { pub fn init() { std::env::var(\"X\"); } pub fn open() { std::fs::read(\"x\"); } }".into()),
            ("cli/src/main.rs".into(), "use memory::Repo; fn run() { Repo::init(); Repo::open(); }".into()),
            ("rms-metadata/rust-crate-alias/cli/memory".into(), "".into()),
        ]);
        let run = |sources| analyze(AnalysisInput { binding: "rust".into(), source_digest: "test".into(), tool_digest: "test".into(), sources,
            semantic_functions: vec![expectation("cli/src/main.rs#run", "effectful", &["environment", "filesystem"])],
            authority_facades: vec![], trusted_external_calls: BTreeSet::new() });
        assert_eq!(run(sources.clone()).result, AnalysisResult::Pass);
        for source in [
            "use other::Repo; fn run() { Repo::init(); Repo::open(); }",
            "mod memory {} fn run() { memory::Repo::init(); memory::Repo::open(); }",
            "fn run<memory>() { memory::Repo::init(); memory::Repo::open(); }",
            "use memory::Repo; fn run<Repo>() { Repo::init(); Repo::open(); }",
        ] {
            let mut changed = sources.clone();
            changed.insert("cli/src/main.rs".into(), source.into());
            assert_eq!(run(changed).result, AnalysisResult::Fail, "{source}");
        }
        let mut changed = sources;
        changed.remove("rms-metadata/rust-crate-alias/cli/memory");
        changed.insert("rms-metadata/rust-crate-alias/another-cli/memory".into(), "".into());
        assert_eq!(run(changed).result, AnalysisResult::Fail);
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
    fn rust_verified_dependency_associated_methods_keep_exact_owner_identity() {
        for (method_body, expected) in [
            ("", AnalysisResult::Pass),
            ("std::fs::read_to_string(\"x\").ok();", AnalysisResult::Fail),
        ] {
            let result = analyze(AnalysisInput {
                binding: "rust".into(), source_digest: "source".into(), tool_digest: "tool".into(),
                sources: BTreeMap::from([
                    ("src/lib.rs".into(), "use decisions::Leaf; fn decide(value: &Leaf) { value.evaluate(); }".into()),
                    ("dependencies/decisions/src/lib.rs".into(), "pub use crate::values::Leaf;".into()),
                    ("dependencies/decisions/src/values.rs".into(), format!("pub struct Leaf; impl Leaf {{ pub fn evaluate(&self) {{ {method_body} }} }} struct Unrelated; impl Unrelated {{ fn evaluate(&self) {{}} }}")),
                    ("rms-metadata/rust-crate-alias/decisions".into(), "dependencies/decisions".into()),
                ]),
                semantic_functions: vec![expectation("src/lib.rs#decide", "pure", &[])], authority_facades: Vec::new(), trusted_external_calls: BTreeSet::new(),
            });
            assert_eq!(result.result, expected, "{result:#?}");
        }
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
    fn rust_qualified_types_keep_same_name_declarations_separate() {
        for (imports, left_effectful, right_effectful, expected) in [
            ("use crate::left::Entry;", false, true, AnalysisResult::Pass),
            ("use crate::right::Entry;", false, true, AnalysisResult::Fail),
            ("use crate::right::Entry;", true, false, AnalysisResult::Pass),
            ("use crate::left::Entry;", true, false, AnalysisResult::Fail),
            ("use crate::left::Entry; use crate::right::Entry;", false, false, AnalysisResult::Fail),
            ("use crate::{left::Entry, right::Entry};", false, false, AnalysisResult::Fail),
            ("struct Entry; impl Entry { fn check(&self) -> bool { true } } use crate::left::Entry;", false, false, AnalysisResult::Fail),
            ("use unknown::Entry;", false, false, AnalysisResult::Fail),
            ("", false, false, AnalysisResult::Fail),
        ] {
            let declaration = |effectful| format!("pub struct Entry; impl Entry {{ pub fn check(&self) -> bool {{ {} true }} }}", if effectful { "std::fs::read_to_string(\"x\");" } else { "" });
            let result = analyze(AnalysisInput {
                binding: "rust".into(), source_digest: "source".into(), tool_digest: "tool".into(),
                sources: BTreeMap::from([
                    ("src/lib.rs".into(), format!("mod left; mod right; {imports} fn decide(values: &[Entry]) {{ values.iter().any(|entry| entry.check()); }} fn direct(entry: &Entry) {{ entry.check(); }}")),
                    ("src/left.rs".into(), declaration(left_effectful)),
                    ("src/right.rs".into(), declaration(right_effectful)),
                    ("tests/independent.rs".into(), declaration(true)),
                ]),
                semantic_functions: vec![expectation("src/lib.rs#decide", "pure", &[]), expectation("src/lib.rs#direct", "pure", &[])],
                authority_facades: Vec::new(), trusted_external_calls: BTreeSet::new(),
            });
            assert_eq!(result.result, expected, "{imports}: {result:#?}");
            assert!(result.functions.iter().all(|function| (function.verdict == FunctionVerdict::Pass) == (expected == AnalysisResult::Pass)), "{imports}: {result:#?}");
        }
        let result = report("rust", "src/lib.rs", "struct Entry; impl Entry { fn check(&self) -> bool { true } } fn decide<Entry>(values: &[Entry]) { values.iter().any(|entry| entry.check()); }", expectation("decide", "pure", &[]));
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
    }

    #[test]
    fn rust_exact_enum_payload_binding_checks_identity_arity_and_effects() {
        let declarations = "struct Endpoint; impl Endpoint { fn check(&self) -> bool { true } fn mutate(&self) -> bool { std::fs::read_to_string(\"x\").is_ok() } } struct Configuration; impl Configuration { fn endpoints(&self) -> &[Endpoint] { &[] } } enum Reply { Resolved(Configuration) } enum Other { Resolved(Configuration) } struct Output { reply: Option<Reply> }";
        for (pattern, operation, prefix, expected) in [
            ("Some(Reply::Resolved(configuration))", "check", "", AnalysisResult::Pass),
            ("Some(Reply::Resolved(configuration))", "mutate", "", AnalysisResult::Fail),
            ("Some(Other::Resolved(configuration))", "check", "", AnalysisResult::Fail),
            ("Some(Reply::Resolved(configuration, extra))", "check", "", AnalysisResult::Fail),
            ("Some(Reply::Missing(configuration))", "check", "", AnalysisResult::Fail),
            ("Some(Reply::Resolved(configuration))", "check", "enum Reply { Resolved(Configuration) }", AnalysisResult::Fail),
        ] {
            let source = format!("{declarations} fn decide(output: Output) {{ {prefix} let {pattern} = output.reply else {{ return; }}; configuration.endpoints().iter().any(|endpoint| endpoint.{operation}()); }}");
            let result = report("rust", "src/lib.rs", &source, expectation("decide", "pure", &[]));
            assert_eq!(result.result, expected, "{pattern} {operation} {prefix}: {result:#?}");
        }
        for (owner, expected) in [("Alias", AnalysisResult::Pass), ("crate::other::Reply", AnalysisResult::Fail)] {
            let result = analyze(AnalysisInput {
                binding: "rust".into(), source_digest: "source".into(), tool_digest: "tool".into(),
                sources: BTreeMap::from([
                    ("src/lib.rs".into(), format!("mod data; mod other; use crate::data::{{Output, Reply as Alias}}; fn decide(output: Output) {{ let Some({owner}::Resolved(configuration)) = output.reply else {{ return; }}; configuration.endpoints().iter().any(|endpoint| endpoint.check()); }}")),
                    ("src/data.rs".into(), declarations.into()),
                    ("src/other.rs".into(), "use crate::data::Configuration; enum Reply { Resolved(Configuration) }".into()),
                ]),
                semantic_functions: vec![expectation("src/lib.rs#decide", "pure", &[])],
                authority_facades: Vec::new(), trusted_external_calls: BTreeSet::new(),
            });
            assert_eq!(result.result, expected, "{owner}: {result:#?}");
        }
    }

    #[test]
    fn rust_bounded_ascii_uppercase_requires_unshadowed_primitive() {
        for source in [
            "fn decide(value: Option<&str>) { value.map(str::to_ascii_uppercase); }",
            "fn decide(value: &str) { str::to_ascii_uppercase(value); }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("decide", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        }
        for source in [
            "struct str; impl str { fn to_ascii_uppercase(_: &Self) { std::fs::read_to_string(\"x\"); } } fn decide(value: &str) { str::to_ascii_uppercase(value); }",
            "fn decide<str>(value: &str) { str::to_ascii_uppercase(value); }",
            "fn decide(value: &str) { use mystery as str; str::to_ascii_uppercase(value); }",
            "use mystery::*; fn decide(value: &str) { str::to_ascii_uppercase(value); }",
            "fn decide(value: Option<&str>) { value.map(Custom::to_ascii_uppercase); }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("decide", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
        }
    }

    #[test]
    fn rust_bounded_callback_slots_preserve_elements_and_effects() {
        let definitions = "struct Leaf; impl Leaf { fn check(&self) -> bool { true } fn items(&self) -> &[Leaf] { todo!() } fn mutate(&self) -> bool { std::fs::read_to_string(\"x\").is_ok() } }";
        for (signature, body, expected) in [
            ("value: Option<&Leaf>", "value.map_or_else(|| false, |leaf| leaf.check());", AnalysisResult::Pass),
            ("values: &[Leaf]", "values.iter().flat_map(|leaf| leaf.items().iter()).count();", AnalysisResult::Pass),
            ("values: &[Leaf]", "values.iter().enumerate().filter_map(|(index, leaf)| leaf.check().then(|| index)).count();", AnalysisResult::Pass),
            ("values: &[Leaf]", "values.iter().fold(false, |accumulator, leaf| accumulator || leaf.check());", AnalysisResult::Pass),
            ("value: Option<&Leaf>", "value.map_or_else(|| { std::fs::read_to_string(\"x\"); false }, |leaf| leaf.check());", AnalysisResult::Fail),
            ("value: Option<&Leaf>", "value.map_or_else(|| false, |leaf| leaf.mutate());", AnalysisResult::Fail),
            ("values: &[Leaf]", "values.iter().flat_map(|leaf| { leaf.mutate(); leaf.items().iter() }).count();", AnalysisResult::Fail),
            ("values: &[Leaf]", "values.iter().enumerate().filter_map(|(_, leaf)| leaf.mutate().then(|| 0)).count();", AnalysisResult::Fail),
            ("values: &[Leaf]", "values.iter().fold(false, |accumulator, leaf| accumulator || leaf.mutate());", AnalysisResult::Fail),
            ("values: &[Leaf]", "values.iter().fold(false, |accumulator, leaf| accumulator.mystery() || leaf.check());", AnalysisResult::Fail),
            ("values: &[Leaf]", "values.iter().flat_map(|leaf| { let leaf = unknown(); leaf.items().iter() }).count();", AnalysisResult::Fail),
            ("values: &[Leaf]", "values.iter().enumerate().filter_map(|(leaf, _)| leaf.check().then(|| 0)).count();", AnalysisResult::Fail),
            ("values: &[Leaf]", "values.iter().enumerate().filter_map(|(_, leaf)| { let leaf = unknown(); leaf.check().then(|| 0) }).count();", AnalysisResult::Fail),
            ("value: Option<&Leaf>, callback: fn(&Leaf) -> bool", "value.map_or_else(|| false, callback);", AnalysisResult::Fail),
            ("values: &[Leaf], callback: fn(bool, &Leaf) -> bool", "values.iter().fold(false, callback);", AnalysisResult::Fail),
            ("values: &[Leaf]", "values.iter().flat_map({ unknown() }).count();", AnalysisResult::Fail),
            ("value: Unknown", "value.map_or_else(|| false, |leaf| leaf.check());", AnalysisResult::Fail),
        ] {
            let source = format!("{definitions} fn decide({signature}) {{ {body} }}");
            let result = report("rust", "src/lib.rs", &source, expectation("decide", "pure", &[]));
            assert_eq!(result.result, expected, "{source}: {result:#?}");
        }
    }

    #[test]
    fn rust_empty_vec_uses_only_exact_mutable_sequence_constraints() {
        let definitions = "struct Leaf; impl Leaf { fn check(&self) -> bool { true } fn mutate(&self) -> bool { std::fs::read_to_string(\"x\").is_ok() } } fn populate(values: &mut Vec<Leaf>) { values.push(Leaf); }";
        for (body, expected) in [
            ("let mut values = Vec::new(); populate(&mut values); values.iter().any(|value| value.check());", AnalysisResult::Pass),
            ("let mut values = Vec::new(); populate(&mut values); values.iter().any(|value| value.mutate());", AnalysisResult::Fail),
            ("let mut values = Vec::new(); mystery(&mut values); values.iter().any(|value| value.check());", AnalysisResult::Fail),
            ("let mut values = Vec::new(); let populate = unknown(); populate(&mut values); values.iter().any(|value| value.check());", AnalysisResult::Fail),
            ("let mut values = Vec::new(); populate(&mut values); let values = unknown(); values.iter().any(|value| value.check());", AnalysisResult::Fail),
        ] {
            let result = report("rust", "src/lib.rs", &format!("{definitions} fn decide() {{ {body} }}"), expectation("decide", "pure", &[]));
            assert_eq!(result.result, expected, "{body}: {result:#?}");
        }
    }

    #[test]
    fn rust_field_optional_and_iterator_facts_preserve_exact_methods() {
        let declarations = "struct Root { leaves: Vec<Leaf>, leaf: Leaf } struct Leaf; impl Leaf { fn next(&self) -> Option<&Leaf> { Some(self) } fn check(&self) -> bool { true } fn mutate(&self) -> bool { std::fs::read_to_string(\"x\").is_ok() } }";
        for (body, expected) in [
            ("root.leaf.check();", AnalysisResult::Pass),
            ("root.leaves.iter().max_by_key(|leaf| leaf.check());", AnalysisResult::Pass),
            ("let latest = root.leaves.iter().max_by_key(|leaf| leaf.check()); let Some(latest) = latest else { return; }; latest.check();", AnalysisResult::Pass),
            ("root.leaf.next().and_then(|leaf| leaf.next()).filter(|leaf| leaf.check());", AnalysisResult::Pass),
            ("root.leaves.iter().max_by_key(|leaf| leaf.mutate());", AnalysisResult::Fail),
            ("root.leaf.next().and_then(|leaf| leaf.next()).filter(|leaf| leaf.mutate());", AnalysisResult::Fail),
            ("let latest = root.leaf.next(); let Some(latest) = latest else { return; }; latest.mutate();", AnalysisResult::Fail),
            ("root.leaf.next().filter(|leaf| { let leaf = unknown(); leaf.check() });", AnalysisResult::Fail),
        ] {
            let result = report("rust", "src/lib.rs", &format!("{declarations} fn decide(root: &Root) {{ {body} }}"), expectation("decide", "pure", &[]));
            assert_eq!(result.result, expected, "{body}: {result:#?}");
        }
    }

    #[test]
    fn rust_external_callbacks_require_closed_types_and_preserve_effects() {
        let io = "use std::io::{self, Read}; fn execute() { io::stdin().read_to_string(&mut String::new()).map_err(|error| error.kind()); }";
        let result = report("rust", "src/lib.rs", io, expectation("execute", "effectful", &["filesystem", "process"]));
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        for source in [
            io.replace("use std::io::{self, Read};", "mod std {} use std::io::{self, Read};"),
            io.replace("error.kind()", "{ let error = unknown(); error.kind() }"),
            io.replace("error.kind()", "{ std::env::var(\"FAULT\"); error.kind() }"),
        ] {
            assert_eq!(report("rust", "src/lib.rs", &source, expectation("execute", "effectful", &["filesystem", "process"])).result, AnalysisResult::Fail, "{source}");
        }
        let git = "use git2::Repository; fn execute(repo: &Repository) { match repo.head() { Ok(head) => { let tree = head.peel_to_commit()?.tree()?; tree.walk(mode, |_prefix, entry| entry.filemode()); }, Err(error) => { error.code(); } } let diff = repo.diff_tree_to_workdir_with_index(None, None)?; diff.print(format, |_delta, _hunk, line| { line.content(); true }); }";
        let run = |source: &str, pin: bool| {
            let mut sources = BTreeMap::from([("src/lib.rs".into(), source.into())]);
            if pin { sources.insert("rms-metadata/rust-external-api/git2-0.20.4".into(), "7b88256088d75a56f8ecfa070513a775dd9107f6530ef14919dac831af9cfe2b".into()); }
            analyze(AnalysisInput { binding: "rust".into(), source_digest: "source".into(), tool_digest: "tool".into(), sources,
                semantic_functions: vec![expectation("execute", "effectful", &["git"])], authority_facades: vec![], trusted_external_calls: BTreeSet::new() })
        };
        let result = run(git, true);
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert_eq!(run(git, false).result, AnalysisResult::Fail);
        let options = "fn execute(root: &std::path::Path) { let repo = git2::Repository::discover(root)?; repo.workdir(); let mut options = git2::StatusOptions::new(); options.include_untracked(true).recurse_untracked_dirs(true).include_ignored(false); let mut diff = git2::DiffOptions::new(); diff.include_untracked(true).recurse_untracked_dirs(true).show_untracked_content(true); }";
        assert_eq!(run(options, true).result, AnalysisResult::Pass);
        assert_eq!(run(options, false).result, AnalysisResult::Fail);
        let diverging = "use git2::Repository; enum Failure { Git(git2::Error) } impl From<git2::Error> for Failure { fn from(value: git2::Error) -> Self { Self::Git(value) } } fn execute(repo: &Repository) -> Result<(), Failure> { let head = match repo.head() { Ok(head) => head, Err(error) => return Err(error.into()) }; head.target(); Ok(()) }";
        assert_eq!(run(diverging, true).result, AnalysisResult::Pass);
        let checked_size = "enum Failure { Invalid } fn execute(content: &str) -> Result<(), Failure> { git2::Repository::open(\".\"); let entry = git2::IndexEntry { file_size: content.len().try_into().map_err(|_| Failure::Invalid)? }; Ok(()) }";
        assert_eq!(run(checked_size, true).result, AnalysisResult::Pass);
        assert_eq!(run(&checked_size.replace("content: &str", "content: Unknown"), true).result, AnalysisResult::Fail);
        assert_eq!(run(&checked_size.replace("|_| Failure::Invalid", "|_| { std::process::abort(); Failure::Invalid }"), true).result, AnalysisResult::Fail);
        let wrapper = "use git2::Repository; enum Failure { Git(git2::Error) } impl From<git2::Error> for Failure { fn from(value: git2::Error) -> Self { Self::Git(value) } } fn execute(repo: &Repository) -> Result<(), Failure> { match repo.head() { Err(error) => return Err(error.into()), Ok(_) => {} } Ok(()) }";
        assert_eq!(run(wrapper, true).result, AnalysisResult::Pass);
        for source in [
            wrapper.replace("Self::Git(value)", "std::env::var(\"FAULT\"); Self::Git(value)"),
            wrapper.replace("use git2::Repository;", "use git2::Repository; use custom::From;"),
            wrapper.replace("return Err(error.into())", "{ let closure = || { return Err(error.into()); }; closure() }"),
            wrapper.replace("use git2::Repository;", "use git2::Repository; fn Err(value: Other) {}"),
        ] { assert_eq!(run(&source, true).result, AnalysisResult::Fail, "{source}"); }
        for source in [
            format!("mod git2 {{}} {git}"),
            git.replace("entry.filemode()", "{ let entry = unknown(); entry.filemode() }"),
            git.replace("entry.filemode()", "{ std::process::abort(); entry.filemode() }"),
            git.replace("entry.filemode()", "entry.unknown_method()"),
            git.replace("entry.filemode()", "CustomCallbackResult"),
            git.replace("|_prefix, entry| entry.filemode()", "unknown_callback"),
            git.replace("use git2::Repository;", "#[cfg(feature = \"git\")] use git2::Repository;"),
        ] { assert_eq!(run(&source, true).result, AnalysisResult::Fail, "{source}"); }
        let path = "use std::path::Path; fn execute(root: &Path) { root.join(\".git\").canonicalize(); }";
        assert_eq!(report("rust", "src/lib.rs", path, expectation("execute", "effectful", &["filesystem"])).result, AnalysisResult::Pass);
        assert_eq!(report("rust", "src/lib.rs", &path.replace("std::path::Path", "custom::Path"), expectation("execute", "pure", &[])).result, AnalysisResult::Fail);
    }

    #[test]
    fn rust_nested_self_results_keep_exact_method_effects() {
        for (method, authorities) in [("check", vec![]), ("cleanup", vec!["filesystem"])] {
            let source = format!("struct Journal; impl Journal {{ fn load() -> Result<Option<Self>, ()> {{ Ok(None) }} fn check(&self) {{}} fn cleanup(&self) {{ std::fs::remove_file(\"journal\"); }} }} fn execute() -> Result<(), ()> {{ let journal = Journal::load()?; if let Some(journal) = journal {{ journal.{method}(); }} Ok(()) }}");
            let result = report("rust", "src/lib.rs", &source, expectation("execute", "effectful", &authorities));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
            assert_eq!(result.functions[0].transitive_authorities, authorities);
        }
        for source in [
            "struct Journal; trait Load { fn load() -> Result<Option<Self>, ()>; } impl Load for Journal { fn load() -> Result<Option<Self>, ()> { Ok(None) } } impl Journal { fn check(&self) {} } fn execute() -> Result<(), ()> { if let Some(journal) = Journal::load()? { journal.check(); } Ok(()) }",
            "struct Journal<T>(T); impl<T> Journal<T> { fn load() -> Result<Option<Self>, ()> { Ok(None) } fn check(&self) {} } fn execute() -> Result<(), ()> { if let Some(journal) = Journal::load()? { journal.check(); } Ok(()) }",
            "struct Journal; impl Journal { fn load() -> Result<Option<Self>, ()> { Ok(None) } fn load() -> Result<Option<Self>, ()> { Ok(None) } fn check(&self) {} } fn execute() -> Result<(), ()> { if let Some(journal) = Journal::load()? { journal.check(); } Ok(()) }",
            "struct Journal; impl Journal { fn load() -> Result<Option<Self>, ()> { Ok(None) } fn check(&self) {} } fn execute() -> Result<(), ()> { if let Some(journal) = Journal::load()? { let journal = unknown(); journal.check(); } Ok(()) }",
        ] {
            assert_eq!(report("rust", "src/lib.rs", source, expectation("execute", "pure", &[])).result, AnalysisResult::Fail, "{source}");
        }
    }

    #[test]
    fn rust_text_byte_predicates_require_a_proven_byte_receiver() {
        let source = "fn decide(raw: &str) -> bool { raw.bytes().enumerate().all(|(index, byte)| { if index == 8 { byte == b'-' } else { byte.is_ascii_hexdigit() } }) }";
        assert_eq!(report("rust", "src/lib.rs", source, expectation("decide", "pure", &[])).result, AnalysisResult::Pass);
        for source in [
            "fn decide(raw: Unknown) { raw.bytes().all(|byte| byte.is_ascii_hexdigit()); }",
            "fn decide(raw: &str) { raw.bytes().all(|byte| { let byte = unknown(); byte.is_ascii_hexdigit() }); }",
            "fn decide(raw: &str) { raw.bytes().all(|byte| { std::fs::remove_file(\"x\"); byte.is_ascii_hexdigit() }); }",
            "struct Byte; impl Byte { fn is_ascii_hexdigit(&self) -> bool { std::fs::remove_file(\"x\"); true } } fn decide(byte: &Byte) { byte.is_ascii_hexdigit(); }",
            "struct str; impl str { fn bytes(&self) -> Unknown { unknown() } } fn decide(raw: &str) { raw.bytes().all(|byte| byte.is_ascii_hexdigit()); }",
        ] {
            assert_eq!(report("rust", "src/lib.rs", source, expectation("decide", "pure", &[])).result, AnalysisResult::Fail, "{source}");
        }
    }

    #[test]
    fn rust_result_and_then_preserves_concrete_callback_types_and_effects() {
        let definitions = "struct Journal; struct Context { journal: Journal } impl Context { fn journal(&self) -> Result<&Journal, ()> { Ok(&self.journal) } } impl Journal { fn check(&self) -> Result<(), ()> { Ok(()) } fn cleanup(&self) -> Result<(), ()> { std::fs::remove_file(\"journal\"); std::env::var(\"FAULT\"); std::process::abort(); Ok(()) } }";
        for (body, authorities) in [
            ("context.journal().and_then(|journal| journal.check());", vec![]),
            ("context.journal().and_then(|journal| journal.cleanup());", vec!["environment", "filesystem", "process"]),
            ("context.journal().and_then(cleanup);", vec!["environment", "filesystem", "process"]),
        ] {
            let source = format!("{definitions} fn cleanup(journal: &Journal) -> Result<(), ()> {{ journal.cleanup() }} fn execute(context: &Context) {{ {body} }}");
            let result = report("rust", "src/lib.rs", &source, expectation("execute", "effectful", &authorities));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
            assert_eq!(result.functions[0].transitive_authorities, authorities);
        }
        for (signature, body) in [
            ("context: &Context", "context.journal().and_then(|journal| { let journal = unknown(); journal.check() });"),
            ("context: &Context, callback: fn(&Journal) -> Result<(), ()>", "context.journal().and_then(callback);"),
            ("context: &Context", "context.journal().and_then(unknown::callback);"),
            ("context: &Context", "context.journal().and_then(make_callback());"),
            ("context: &Context", "context.journal().map_err(|error| error.check());"),
            ("context: Unknown", "context.journal().and_then(|journal| journal.check());"),
        ] {
            let source = format!("{definitions} fn execute({signature}) {{ {body} }}");
            let result = report("rust", "src/lib.rs", &source, expectation("execute", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
        }
        let shadowed = "struct Result<T, E>(T, E); struct Journal; fn execute(value: Result<Journal, ()>) { value.and_then(|journal| journal.check()); }";
        assert_eq!(report("rust", "src/lib.rs", shadowed, expectation("execute", "pure", &[])).result, AnalysisResult::Fail);
    }

    #[test]
    fn rust_uuid_v4_requires_exact_external_identity_and_is_never_pure() {
        for source in [
            "fn generate() { uuid::Uuid::new_v4(); }",
            "fn generate() { ::uuid::Uuid::new_v4(); }",
            "use uuid::Uuid; fn generate() { Uuid::new_v4(); }",
            "use uuid::Uuid as OperationId; fn generate() { OperationId::new_v4(); }",
            "use uuid as ids; fn generate() { ids::Uuid::new_v4(); }",
        ] {
            let effectful = report("rust", "src/lib.rs", source, expectation("generate", "effectful", &["randomness"]));
            assert_eq!(effectful.result, AnalysisResult::Pass, "{source}: {effectful:#?}");
            assert_eq!(effectful.functions[0].transitive_authorities, vec!["randomness"]);
            assert_eq!(report("rust", "src/lib.rs", source, expectation("generate", "pure", &[])).result, AnalysisResult::Fail);
        }
        for source in [
            "fn generate() { uuid::Uuid::new_v7(); }",
            "mod uuid { pub struct Uuid; impl Uuid { pub fn new_v4() { unknown(); } } } fn generate() { uuid::Uuid::new_v4(); }",
            "use other::Uuid; fn generate() { Uuid::new_v4(); }",
            "use other as uuid; fn generate() { uuid::Uuid::new_v4(); }",
            "extern crate other as uuid; fn generate() { uuid::Uuid::new_v4(); }",
            "#[cfg(feature=\"maybe\")] use uuid::Uuid; fn generate() { Uuid::new_v4(); }",
            "use uuid::Uuid; use other::Uuid; fn generate() { Uuid::new_v4(); }",
            "use unknown::*; fn generate() { uuid::Uuid::new_v4(); }",
            "use uuid as ids; mod ids {} fn generate() { ids::Uuid::new_v4(); }",
            "use uuid::Uuid; fn generate<Uuid>() { Uuid::new_v4(); }",
            "use uuid::Uuid; fn generate() { struct Uuid; Uuid::new_v4(); }",
            "fn generate() { use other as uuid; uuid::Uuid::new_v4(); }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("generate", "effectful", &["randomness"]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
            assert!(!result.functions[0].transitive_authorities.contains(&"randomness".to_string()), "{source}: {result:#?}");
        }
        let mixed = report("rust", "src/lib.rs", "fn generate() { uuid::Uuid::new_v4(); std::env::var(\"FAULT\"); std::process::abort(); }", expectation("generate", "effectful", &["randomness", "environment", "process"]));
        assert_eq!(mixed.result, AnalysisResult::Pass, "{mixed:#?}");
        let unknown = report("rust", "src/lib.rs", "fn generate() { uuid::Uuid::new_v4(); unverified::call(); }", expectation("generate", "effectful", &["randomness"]));
        assert_eq!(unknown.result, AnalysisResult::Fail);
        assert_eq!(unknown.functions[0].unresolved_calls, vec!["unverified::call"]);
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
    fn swift_string_split_collection_capture_is_typed_and_shadowing_sensitive() {
        for (argument_type, inner, expected) in [
            ("String", "func identity() -> String { fields[1] }; return identity()", AnalysisResult::Pass),
            ("Custom", "func identity() -> String { fields[1] }; return identity()", AnalysisResult::Fail),
            ("String", "func identity(fields: Custom) -> String { fields[1] }; return identity(fields: unknown())", AnalysisResult::Fail),
            ("String", "return { (fields: Custom) in fields[1] }(Custom())", AnalysisResult::Fail),
        ] {
            let source = format!("func parse(_ raw: {argument_type}) -> String {{ let fields = raw.split(separator: \"|\", omittingEmptySubsequences: false).map(String.init); {inner} }}");
            let result = report("swift", "Sources/Parser.swift", &source, expectation("parse", "pure", &[]));
            assert_eq!(result.result, expected, "{source}: {result:#?}");
        }
    }

    #[test]
    fn swift_checked_continuation_requires_literal_unshadowed_call_and_keeps_closure_effects() {
        for (source, expected, unresolved) in [
            ("func execute() async { await withCheckedContinuation { continuation in print(\"effect\") } }", AnalysisResult::Fail, false),
            ("func execute(callback: Callback) async { await withCheckedContinuation(callback) }", AnalysisResult::Fail, true),
            ("func execute(withCheckedContinuation: Callback) async { await withCheckedContinuation { continuation in } }", AnalysisResult::Fail, true),
            ("func withCheckedContinuation(_ callback: Callback) { print(\"effect\") }; func execute() async { await withCheckedContinuation { continuation in } }", AnalysisResult::Fail, false),
        ] {
            let result = report("swift", "Sources/Executor.swift", source, expectation("execute", "pure", &[]));
            assert_eq!(result.result, expected, "{source}: {result:#?}");
            assert_eq!(result.functions[0].unresolved_calls.contains(&"withCheckedContinuation".to_string()), unresolved, "{source}: {result:#?}");
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
                ("src/consumer.rs".to_string(), "use provider::Provider; fn project(value: &Provider) -> u16 { value.port() } fn decide(values: &[Provider]) -> Vec<u16> { values.iter().map(project).collect() }".to_string()),
                ("dependencies/provider/src/lib.rs".to_string(), "pub struct Provider { port: u16 } impl Provider { pub fn port(&self) -> u16 { self.port } }".to_string()),
                ("rms-metadata/rust-crate-alias/provider".to_string(), "dependencies/provider".to_string()),
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
    fn rust_retain_preserves_element_identity_and_callback_authority() {
        let value = "struct Receipt; impl Receipt { fn evidence(&self) -> bool { true } } ";
        for body in [
            "fn run(values: &mut Vec<Receipt>) { values.retain(|r| r.evidence()); }",
            "fn keep(r: &Receipt) -> bool { r.evidence() } fn run(values: &mut Vec<Receipt>) { values.retain(keep); }",
            "fn retain() { std::fs::read(\"x\"); } fn run(values: &mut Vec<Receipt>) { values.retain(|r| r.evidence()); }",
            "fn run(values: &Vec<Receipt>) -> Option<bool> { values.iter().find_map(|r| Some(r.evidence())) }",
        ] {
            let result = report("rust", "src/lib.rs", &format!("{value}{body}"), expectation("run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
            assert!(result.functions[0].transitive_authorities.is_empty());
        }
        for (body, authority) in [
            ("fn run(values: &mut Vec<Receipt>) { values.retain(|r| { std::fs::read(\"x\"); r.evidence() }); }", "filesystem"),
            ("fn keep(r: &Receipt) -> bool { std::fs::read(\"x\"); r.evidence() } fn run(values: &mut Vec<Receipt>) { values.retain(keep); }", "filesystem"),
            ("fn run(values: &mut Vec<Receipt>, keep: impl FnMut(&Receipt) -> bool) { values.retain(keep); }", "dynamic-dispatch"),
            ("fn run(values: &mut Vec<Receipt>, keep: impl Fn(&Receipt) -> bool) { values.retain(|r| keep(r)); }", "dynamic-dispatch"),
            ("fn run(values: &Vec<Receipt>, keep: impl FnMut(&Receipt) -> Option<bool>) { values.iter().find_map(keep); }", "dynamic-dispatch"),
            ("fn run(values: &Vec<Receipt>) { values.iter().find_map(|r| { std::fs::read(\"x\"); Some(r.evidence()) }); }", "filesystem"),
            ("struct Custom; impl Custom { fn retain(&mut self, f: impl Fn(&Receipt) -> bool) { std::fs::read(\"x\"); } } fn run(values: &mut Custom) { values.retain(|r| r.evidence()); }", "filesystem"),
        ] {
            let result = report("rust", "src/lib.rs", &format!("{value}{body}"), expectation("run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
            assert!(result.functions[0].transitive_authorities.contains(&authority.into()), "{result:#?}");
        }
        let source = "#[derive(Clone)] struct Receipt; impl Receipt { fn evidence(&self) -> bool { true } } #[derive(Clone)] struct Context { receipts: Vec<Receipt> } fn run(context: &Context) { let mut c = context.clone(); c.receipts.retain(|r| r.evidence()); }";
        let result = report("rust", "src/lib.rs", source, expectation("run", "pure", &[]));
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        let source = source.replace("fn run(context:", "impl Context { fn clone(&self) -> Context { std::fs::read(\"x\"); Context { receipts: vec![] } } } fn run(context:");
        let result = report("rust", "src/lib.rs", &source, expectation("run", "pure", &[]));
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.contains(&"filesystem".into()), "{result:#?}");
    }

    #[test]
    fn rust_named_character_predicate_requires_primitive_identity() {
        for source in [
            "fn run(value: String) -> bool { value.chars().any(char::is_control) }",
            "fn run(value: &str) -> bool { value.chars().all(char::is_control) }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
            assert!(result.functions[0].transitive_authorities.is_empty());
            assert!(result.functions[0].unresolved_calls.is_empty());
        }
        for source in [
            "mod char { pub fn is_control(_: std::primitive::char) -> bool { std::fs::read(\"x\"); true } } fn run(value: &str) -> bool { value.chars().any(char::is_control) }",
            "struct Custom; impl Custom { fn is_control(_: char) -> bool { std::fs::read(\"x\"); true } } use Custom as char; fn run(value: &str) -> bool { value.chars().any(char::is_control) }",
            "fn run(value: &str) -> bool { use custom as char; value.chars().any(char::is_control) }",
            "fn run<char>(value: &str) -> bool { value.chars().any(char::is_control) }",
            "fn run(value: &str, predicate: impl FnMut(char) -> bool) -> bool { value.chars().any(predicate) }",
            "fn is_control(_: char) -> bool { std::fs::read(\"x\"); true } fn run(value: &str) -> bool { value.chars().any(is_control) }",
            "fn run(value: &str) -> bool { value.bytes().any(char::is_control) }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
        }
    }

    #[test]
    fn rust_character_predicates_preserve_purity_and_callback_authority() {
        let source = "fn valid_id(value: &str) -> bool { !value.is_empty() && value.len() <= 256 && !value.chars().any(|c| c.is_whitespace() || c.is_control()) } fn decide(value: &str) -> bool { valid_id(value) }";
        let result = report("rust", "src/lib.rs", source, expectation("decide", "pure", &[]));
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0].transitive_authorities.is_empty());
        assert!(result.functions[0].unresolved_calls.is_empty());

        for source in [
            "struct Custom; impl Custom { fn is_control(&self) -> bool { std::fs::read(\"x\").is_ok() } } fn decide(value: &Custom) -> bool { value.is_control() }",
            "struct Custom; impl Custom { fn chars(&self) -> Vec<Custom> { vec![] } fn is_control(&self) -> bool { std::fs::read(\"x\").is_ok() } } fn decide(value: &Custom) -> bool { value.chars().iter().any(|c| c.is_control()) }",
            "fn decide(value: &str) -> bool { value.chars().any(|c| { std::fs::read(\"x\"); c.is_control() }) }",
            "fn decide(value: &str, callback: impl Fn(char) -> bool) -> bool { value.chars().any(|c| callback(c) || c.is_control()) }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("decide", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
            assert!(!result.functions[0].transitive_authorities.is_empty(), "{result:#?}");
        }
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
        assert!(result.functions[0].unresolved_calls.is_empty());
        assert_eq!(result.functions[0].transitive_authorities, vec!["dynamic-dispatch"]);
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
    fn python_default_callable_adds_known_effects_without_blessing_injection() {
        let source = "import os\nfrom pathlib import Path\nfrom urllib.request import Request, urlopen\nfrom collections.abc import Callable\ndef default_open(request):\n    Path('ca.pem').is_file()\n    os.environ.get('SSL_CERT_FILE')\n    return urlopen(request)\ndef execute(value, opener: Callable = default_open):\n    request = Request(value)\n    response = opener(request)\n    return getattr(response, 'status', type(response))\n";
        let result = report(
            "python",
            "src/adapter.py",
            source,
            expectation(
                "execute",
                "effectful",
                &["filesystem", "environment", "network", "dynamic-dispatch"],
            ),
        );
        for authority in ["filesystem", "environment", "network"] {
            assert!(
                result.functions[0]
                    .transitive_authorities
                    .contains(&authority.to_string()),
                "{result:#?}"
            );
        }
        assert!(result.functions[0].unresolved_calls.is_empty(), "{result:#?}");
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        assert!(result.functions[0]
            .direct_calls
            .contains(&"python-stdlib.urllib.request.Request".to_string()));
    }

    #[test]
    fn python_dispatch_refinement_keeps_unknown_and_rebound_calls_open() {
        for source in [
            "def run(value):\n    return mystery(value)\n",
            "def run(callback, value):\n    callback = mystery\n    return callback(value)\n",
            "def run(callback, value):\n    def nested(callback):\n        return callback(value)\n    return callback(value)\n",
            "def getattr(value):\n    return mystery(value)\ndef run(value):\n    return getattr(value)\n",
            "type = mystery\ndef run(value):\n    return type(value)\n",
        ] {
            let result = report("python", "src/adapter.py", source,
                expectation("run", "effectful", &["dynamic-dispatch"]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
            assert!(!result.functions[0].unresolved_calls.is_empty(), "{source}: {result:#?}");
        }
        for source in [
            "def run(callback, value):\n    return callback(value)\n",
            "def run(value):\n    return getattr(value, 'status')\n",
            "def run(value):\n    return type(value)\n",
        ] {
            let result = report("python", "src/adapter.py", source,
                expectation("run", "effectful", &["dynamic-dispatch"]));
            assert_eq!(result.result, AnalysisResult::Pass, "{source}: {result:#?}");
        }
    }

    #[test]
    fn python_urlopen_shadowing_does_not_gain_stdlib_identity() {
        let result = report("python", "src/adapter.py", "from urllib.request import urlopen\ndef execute(urlopen, value):\n    return urlopen(value)\n", expectation("execute", "pure", &[]));
        assert_eq!(result.result, AnalysisResult::Fail);
        assert!(!result.functions[0]
            .transitive_authorities
            .contains(&"network".to_string()));
        let result = analyze(AnalysisInput {
            binding: "python".into(), source_digest: "source".into(), tool_digest: "tool".into(),
            sources: BTreeMap::from([
                ("src/adapter.py".into(), "from urllib.request import urlopen\ndef execute(value):\n    return urlopen(value)\n".into()),
                ("src/urllib/request.py".into(), "def urlopen(value):\n    return mystery(value)\n".into()),
            ]),
            semantic_functions: vec![expectation("src/adapter.py#execute", "pure", &[])], authority_facades: Vec::new(), trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Fail);
        assert!(!result.functions[0]
            .transitive_authorities
            .contains(&"network".to_string()));
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
    fn swift_guarded_foundation_values_are_pure_without_blessing_shadowing() {
        let uuid = "import Foundation\nfunc parse(_ value: String?) -> Bool { guard let value, let parsed = UUID(uuidString: value) else { return false }; return parsed.uuidString.lowercased() == value }";
        let dictionary = "import Foundation\nfunc parse(_ payload: String) -> Any? { let trimmed = payload.trimmingCharacters(in: .whitespacesAndNewlines); guard !trimmed.isEmpty else { return nil }; guard let data = trimmed.data(using: .utf8), let object = try? JSONSerialization.jsonObject(with: data), let fields = object as? [String: Any] else { return nil }; return fields[\"reason\"] }";
        for source in [uuid, dictionary] {
            let result = report("swift", "Sources/Parser.swift", source, expectation("parse", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        }
        for source in [
            format!("struct UUID {{}}\n{uuid}"),
            uuid.replace("return parsed.uuidString", "let parsed = unknown(); return parsed.uuidString"),
            dictionary.replace("[String: Any]", "CustomDictionary"),
            dictionary.replace("return fields[", "let fields = unknown(); return fields["),
            dictionary.replace("_ payload: String", "_ payload: CustomString"),
            dictionary.replace(".data(using: .utf8)", ".data(using: unknown())"),
            format!("extension String {{ func data(using: Int) -> Int {{ unknown() }} }}\n{dictionary}"),
        ] {
            let result = report("swift", "Sources/Parser.swift", &source, expectation("parse", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
        }
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
    fn rust_exact_source_path_precedes_dependency_suffixes() {
        let result = analyze(AnalysisInput {
            binding: "rust".into(), source_digest: "test".into(), tool_digest: "test".into(),
            sources: BTreeMap::from([
                ("src/transition.rs".into(), "fn run() { transition_record(); } fn transition_record() {}".into()),
                ("dependencies/other/src/transition.rs".into(), "fn transition_record() { std::fs::read_to_string(\"file\"); }".into()),
            ]),
            semantic_functions: vec![expectation("src/transition.rs#run", "pure", &[])],
            authority_facades: Vec::new(), trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
    }

    #[test]
    fn rust_local_struct_construction_preserves_exact_receiver() {
        for source in [
            "struct Writer(Vec<u8>); impl Writer { fn policy(&self) {} } fn policy() { std::fs::read_to_string(\"file\"); } fn run() { let w = Writer(Vec::new()); w.policy(); }",
            "struct Reader<'a> { bytes: &'a [u8] } impl Reader<'_> { fn policy(&self) {} } fn policy() { std::fs::read_to_string(\"file\"); } fn run(bytes: &[u8]) { let r = Reader { bytes }; r.policy(); }",
        ] {
            let result = report("rust", "src/codec.rs", source, expectation("src/codec.rs#run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        }
        for source in [
            "struct Writer(Vec<u8>); impl Writer { fn policy(&self) { std::fs::read_to_string(\"file\"); } } fn run() { let w = Writer(Vec::new()); w.policy(); }",
            "struct Writer(Vec<u8>); fn run(Writer: impl Fn() -> Unknown) { let w = Writer(); w.unknown(); }",
            "struct Writer(Vec<u8>); struct Writer; fn run() { let w = Writer(Vec::new()); w.unknown(); }",
            "struct Writer(Vec<u8>); fn Writer() -> Unknown { todo!() } fn run() { let w = Writer(); w.unknown(); }",
            "struct Reader { field: u8 } fn run() { struct Reader; let r = Reader {}; r.unknown(); }",
            "struct Writer<T>(T); fn run() { let w = Writer(0); w.unknown(); }",
        ] {
            let result = report("rust", "src/codec.rs", source, expectation("src/codec.rs#run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
        }
    }

    #[test]
    fn rust_integer_byte_conversions_require_unshadowed_primitives() {
        for source in [
            "fn run(n: u32) -> u32 { u32::from_be_bytes(n.to_be_bytes()) }",
            "fn run(n: u64) -> u64 { u64::from_le_bytes(u64::to_le_bytes(n)) }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("src/lib.rs#run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        }
        for source in [
            "struct u32; impl u32 { fn from_be_bytes(_: [u8; 4]) { std::fs::read_to_string(\"file\"); } } fn run() { u32::from_be_bytes([0;4]); }",
            "fn run<u32>() { u32::from_be_bytes([0;4]); }",
            "use unknown::*; fn run(n: u32) { n.to_be_bytes(); }",
            "fn run(value: impl Unknown) { value.to_be_bytes(); }",
            "fn run() { type u32 = Unknown; u32::from_be_bytes([0;4]); }",
        ] {
            let result = report("rust", "src/lib.rs", source, expectation("src/lib.rs#run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
        }
        for (representation, expected) in [("struct Domain;", AnalysisResult::Pass), ("pub struct u32;", AnalysisResult::Fail)] {
            let result = analyze(AnalysisInput {
                binding: "rust".into(), source_digest: "test".into(), tool_digest: "test".into(),
                sources: BTreeMap::from([
                    ("src/codec.rs".into(), "use crate::representation::*; fn run(n: u32) { u32::from_be_bytes(n.to_be_bytes()); }".into()),
                    ("src/representation.rs".into(), representation.into()),
                ]),
                semantic_functions: vec![expectation("src/codec.rs#run", "pure", &[])],
                authority_facades: Vec::new(), trusted_external_calls: BTreeSet::new(),
            });
            assert_eq!(result.result, expected, "{result:#?}");
        }
    }

    #[test]
    fn rust_inherent_self_calls_do_not_reach_same_leaf_probe_helpers() {
        let source = "struct Reader; impl Reader { fn policy(&self) -> bool { true } fn run(&self) -> bool { self.policy() } } fn policy() { std::fs::read_to_string(\"file\"); }";
        let result = report("rust", "src/codec.rs", source, expectation("src/codec.rs#Reader::run", "pure", &[]));
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        for source in [
            source.replace("fn policy(&self) -> bool { true }", "fn policy(&self) -> bool { std::fs::read_to_string(\"file\").is_ok() }"),
            "struct Reader; impl Unknown for Reader { fn run(&self) { self.policy(); } } fn policy() {}".into(),
            "struct Reader<T>(T); impl<T> Reader<T> { fn run(&self) { self.policy(); } } fn policy() {}".into(),
            "struct Reader; impl Reader { fn run(&self) { self.policy(); } fn policy(&self) {} } impl Reader { fn policy(&self) {} }".into(),
        ] {
            let result = report("rust", "src/codec.rs", &source, expectation("src/codec.rs#Reader::run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
        }
    }

    #[test]
    fn rust_last_mut_requires_a_concrete_unshadowed_sequence() {
        for source in [
            "fn run(values: &mut Vec<u8>) { let _ = values.last_mut(); }",
            "fn run(values: &mut [u8]) { let _ = values.last_mut(); }",
            "fn run() { let mut values: Vec<Option<std::collections::BTreeSet<String>>> = Vec::new(); let _ = values.last_mut().and_then(Option::as_mut); }",
            "struct Item; impl Item { fn score(&self) -> bool { true } } fn run(values: &mut Vec<Item>) { values.last_mut().is_some_and(|item| item.score()); }",
        ] {
            let result = report("rust", "src/test.rs", source, expectation("src/test.rs#run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{source}: {result:#?}");
        }
        for source in [
            "trait Unknown { fn last_mut(&mut self); } fn run(values: &mut impl Unknown) { values.last_mut(); }",
            "mod std { pub mod vec { pub struct Vec<T>(T); } } use std::vec::Vec; fn run(values: &mut Vec<u8>) { values.last_mut(); }",
            "fn run<T>(values: &mut Vec<T>) { values.last_mut().is_some_and(|item| item.unknown()); }",
            "struct Vec<T>(T); impl<T> Vec<T> { fn last_mut(&mut self) { std::fs::read_to_string(\"file\"); } } fn run(values: &mut Vec<u8>) { values.last_mut(); }",
            "struct Local; impl Local { fn last_mut(&mut self) { std::fs::read_to_string(\"file\"); } } fn run(values: &mut Local) { values.last_mut(); }",
            "struct Item; impl Item { fn score(&self) -> bool { std::fs::read_to_string(\"file\").is_ok() } } fn run(values: &mut Vec<Item>) { values.last_mut().is_some_and(|item| item.score()); }",
        ] {
            let result = report("rust", "src/test.rs", source, expectation("src/test.rs#run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
        }
    }

    #[test]
    fn rust_array_tuple_callback_preserves_exact_receiver() {
        let source = r#"
struct Readiness;
impl Readiness { fn owner(&self) -> bool { true } }
struct Data { facts: [Option<(Readiness, u64)>; 4] }
fn ready(data: &Data) -> bool {
    data.facts.iter().enumerate().all(|(_, fact)| {
        fact.as_ref().is_some_and(|(readiness, _)| readiness.owner())
    })
}
"#;
        let result = report("rust", "src/array.rs", source, expectation("src/array.rs#ready", "pure", &[]));
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        let effectful = source.replace("fn owner(&self) -> bool { true }", "fn owner(&self) -> bool { std::fs::read_to_string(\"file\").is_ok() }");
        let result = report("rust", "src/array.rs", &effectful, expectation("src/array.rs#ready", "pure", &[]));
        assert_eq!(result.result, AnalysisResult::Fail);
        assert!(result.functions[0].transitive_authorities.contains(&"filesystem".into()), "{result:#?}");
        let unknown = source.replace("(Readiness, u64)", "(Readiness, Box<dyn Fn() -> bool>)")
            .replace("|(readiness, _)| readiness.owner()", "|(_, callback)| callback()");
        let result = report("rust", "src/array.rs", &unknown, expectation("src/array.rs#ready", "pure", &[]));
        assert_eq!(result.result, AnalysisResult::Fail);
        assert!(result.functions[0].transitive_authorities.contains(&"dynamic-dispatch".into()), "{result:#?}");
    }

    #[test]
    fn rust_standard_poll_fn_keeps_closure_effects_and_unknown_callbacks() {
        for root in ["std", "core"] {
            let source = format!("fn run() {{ ::{root}::future::poll_fn(|_| ::{root}::task::Poll::<()>::Pending); }}");
            let result = report("rust", "src/poll.rs", &source, expectation("src/poll.rs#run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        }
        for source in [
            "fn run() { ::std::future::poll_fn(|_| { std::fs::read_to_string(\"file\"); }); }",
            "fn run(callback: impl FnMut()) { ::std::future::poll_fn(callback); }",
            "fn run(callback: impl FnMut()) { ::std::future::poll_fn(|_| callback()); }",
            "extern crate custom as std; fn run() { ::std::future::poll_fn(|_| true); }",
            "mod std { pub mod future { pub fn poll_fn<T>(_: T) {} } } fn run() { std::future::poll_fn(|_| true); }",
        ] {
            let result = report("rust", "src/poll.rs", source, expectation("src/poll.rs#run", "pure", &[]));
            assert_eq!(result.result, AnalysisResult::Fail, "{source}: {result:#?}");
        }
    }

    #[test]
    fn rust_standard_poll_fn_rejects_cross_file_external_rebinding() {
        let result = analyze(AnalysisInput {
            binding: "rust".into(),
            source_digest: "source".into(),
            tool_digest: "tool".into(),
            sources: BTreeMap::from([
                ("src/lib.rs".into(), "extern crate custom as std; mod driver;".into()),
                ("src/driver.rs".into(), "fn run() { ::std::future::poll_fn(|_| ::core::task::Poll::<()>::Pending); }".into()),
            ]),
            semantic_functions: vec![expectation("src/driver.rs#run", "pure", &[])],
            authority_facades: Vec::new(),
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
        assert!(result.functions[0].unresolved_calls.contains(&"std::future::poll_fn".into()), "{result:#?}");
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
    fn swift_callable_parameter_invocation_is_dynamic_not_unresolved_or_pure() {
        let source = "typealias Operation = @Sendable (Int) async -> Int\nfunc execute(_ value: Int, using operation: Operation) async -> Int { await operation(value) }";
        let dynamic = report("swift", "Executor.swift", source, expectation("execute", "effectful", &["dynamic-dispatch"]));
        assert_eq!(dynamic.result, AnalysisResult::Pass, "{dynamic:#?}");
        assert!(dynamic.functions[0].unresolved_calls.is_empty());
        let pure = report("swift", "Executor.swift", source, expectation("execute", "pure", &[]));
        assert_eq!(pure.result, AnalysisResult::Fail);
        let named_global = format!("func operation(_ value: Int) -> Int {{ value }}\n{source}");
        let dynamic = report("swift", "Executor.swift", &named_global, expectation("execute", "effectful", &["dynamic-dispatch"]));
        assert_eq!(dynamic.result, AnalysisResult::Pass, "{dynamic:#?}");
        for source in [
            "func execute() { operation(1) }",
            "func execute(operation: Operation) { let operation = missing; operation(1) }",
            "func execute(operation: Operation) { let (operation, other) = missing; operation(1) }",
            "func execute(operation: Operation) { let local = { operation in operation(1) }; local(operation) }",
            "func execute(operation: Operation) { func operation(_ value: Int) { print(value) }; operation(1) }",
        ] {
            let result = report("swift", "Executor.swift", source, expectation("execute", "effectful", &["dynamic-dispatch"]));
            assert_ne!(result.result, AnalysisResult::Pass, "{source}: {result:#?}");
        }
    }

    #[test]
    fn swift_facade_preserves_callback_dispatch_and_unresolved_obligations() {
        for (body, expected) in [
            ("await operation(value)", AnalysisResult::Pass),
            ("missing(value)", AnalysisResult::Fail),
        ] {
            let result = analyze(AnalysisInput {
                binding: "swift".into(), source_digest: "source".into(), tool_digest: "tool".into(),
                sources: BTreeMap::from([("Executor.swift".into(), format!("typealias Operation = @Sendable (Int) async -> Int\nfunc execute(_ value: Int) -> Int {{ value }}\nfunc execute(_ value: Int, using operation: Operation) async -> Int {{ {body} }}\nfunc drive(_ value: Int, using operation: Operation) async -> Int {{ await execute(value, using: operation) }}"))]),
                semantic_functions: vec![expectation("drive", "effectful", &["device", "dynamic-dispatch"])],
                authority_facades: vec![AuthorityFacade { authority: "device".into(), symbol: "Executor.swift#execute(_:Int,using:Operation)".into() }],
                trusted_external_calls: BTreeSet::new(),
            });
            assert_eq!(result.result, expected, "{result:#?}");
            if expected == AnalysisResult::Pass {
                assert_eq!(result.functions[0].transitive_authorities, vec!["device", "dynamic-dispatch"]);
            } else {
                assert!(result.functions[0].unresolved_calls.contains(&"missing".to_string()));
            }
        }
    }

    #[test]
    fn swift_stored_and_trailing_argument_callbacks_keep_dynamic_authority() {
        for source in [
            "class Completion { private let deliver: @Sendable () -> Void; func execute() { deliver() } }",
            "func execute(operation: Operation) { operation(1) { FileManager.default.remove_file(\"token\") } }",
        ] {
            let authorities = if source.contains("FileManager") { vec!["dynamic-dispatch", "filesystem"] } else { vec!["dynamic-dispatch"] };
            let result = report("swift", "Executor.swift", source, expectation("execute", "effectful", &authorities));
            assert_eq!(result.result, AnalysisResult::Pass, "{source}: {result:#?}");
        }
        let source = "class Completion { private let deliver: @Sendable () -> Void; func execute() { let deliver = missing; deliver() } }";
        let result = report("swift", "Executor.swift", source, expectation("execute", "effectful", &["dynamic-dispatch"]));
        assert_eq!(result.result, AnalysisResult::Fail, "{result:#?}");
    }

    #[test]
    fn raw_authority_facades_preserve_categories_and_migrate_with_exact_containment() {
        let result = analyze(AnalysisInput {
            binding: "rust".into(), source_digest: "source".into(), tool_digest: "tool".into(),
            sources: BTreeMap::from([("src/io.rs".into(), "fn execute() { std::fs::read_to_string(\"x\").ok(); std::env::var(\"X\").ok(); }".into())]),
            semantic_functions: vec![expectation("src/io.rs#execute", "effectful", &["environment", "filesystem"])],
            authority_facades: ["environment", "filesystem"].into_iter().map(|authority| AuthorityFacade { authority: authority.into(), symbol: "src/io.rs#execute".into() }).collect(),
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        let binding = serde_yaml::from_str("spec: rms/implementation/v0.1\narchitecture:\n  authority_bindings:\n  - {authority: filesystem, safe_facade: 'src/io.rs#execute'}\n  - {authority: environment, safe_facade: 'src/io.rs#execute'}\nsemantic_functions:\n- {id: subject, kind: effect-executor, symbol: 'src/io.rs#execute', purity: effectful}\n").unwrap();
        assert!(crate::binding_migration::plan(&binding, &result, "v0.2").is_ok());
    }

    #[test]
    fn nested_rust_facades_preserve_inner_authorities_and_unknown_calls() {
        let make = |inner: &str| analyze(AnalysisInput {
            binding: "rust".into(), source_digest: "source".into(), tool_digest: "tool".into(),
            sources: BTreeMap::from([
                ("src/lib.rs".into(), "mod disk; fn run() { std::env::var(\"X\").ok(); disk::execute(); }".into()),
                ("src/disk.rs".into(), format!("pub fn execute() {{ std::fs::read(\"x\").ok(); {inner} }}")),
            ]),
            semantic_functions: vec![expectation("src/lib.rs#run", "effectful", &["cli", "repository"]), expectation("src/disk.rs#execute", "effectful", &["repository"])],
            authority_facades: vec![AuthorityFacade { authority: "cli".into(), symbol: "src/lib.rs#run".into() }, AuthorityFacade { authority: "repository".into(), symbol: "src/disk.rs#execute".into() }],
            trusted_external_calls: BTreeSet::new(),
        });
        let result = make("");
        assert_eq!(result.result, AnalysisResult::Pass, "{result:#?}");
        let unknown = make("unverified::perform();");
        assert_eq!(unknown.result, AnalysisResult::Fail);
        assert!(unknown.functions.iter().all(|function| function.unresolved_calls.contains(&"unverified::perform".to_string())));
        let separate = analyze(AnalysisInput {
            binding: "rust".into(), source_digest: "source".into(), tool_digest: "tool".into(),
            sources: BTreeMap::from([("src/lib.rs".into(), "fn cli() { worker(); } fn worker() { std::fs::read(\"x\").ok(); }".into())]),
            semantic_functions: vec![expectation("src/lib.rs#worker", "effectful", &["filesystem"])],
            authority_facades: vec![AuthorityFacade { authority: "cli-stdio".into(), symbol: "src/lib.rs#cli".into() }],
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(separate.functions[0].transitive_authorities, vec!["filesystem"]);
        assert_eq!(separate.result, AnalysisResult::Pass);
    }

    #[test]
    fn immediately_invoked_rust_closures_keep_body_effects_and_unknown_calls() {
        for (body, purity, authorities, passes) in [
            ("(|| 1)();", "pure", vec![], true),
            ("((|| { std::fs::read(\"x\"); }))();", "effectful", vec!["filesystem"], true),
            ("(|| unverified::perform())();", "pure", vec![], false),
            ("(|callback| callback())(unknown);", "pure", vec![], false),
            ("(factory())();", "pure", vec![], false),
        ] {
            let result = report("rust", "src/lib.rs", &format!("fn subject() {{ {body} }}"),
                expectation("src/lib.rs#subject", purity, &authorities));
            assert_eq!(result.result == AnalysisResult::Pass, passes, "{body}: {result:#?}");
        }
    }

    #[test]
    fn named_facade_and_dynamic_witness_are_not_competing_remappings() {
        for secondary in ["dynamic-dispatch", "other-device", "filesystem"] {
            let result = analyze(AnalysisInput {
                binding: "swift".into(), source_digest: "source".into(), tool_digest: "tool".into(),
                sources: BTreeMap::from([("Executor.swift".into(), "func execute(using operation: Operation) { operation() }".into())]),
                semantic_functions: vec![expectation("execute", "effectful", &["dynamic-dispatch"])],
                authority_facades: ["device", secondary].into_iter().map(|authority| AuthorityFacade { authority: authority.into(), symbol: "Executor.swift#execute(using:Operation)".into() }).collect(),
                trusted_external_calls: BTreeSet::new(),
            });
            assert_eq!(result.result, if secondary == "dynamic-dispatch" { AnalysisResult::Pass } else { AnalysisResult::Fail }, "{result:#?}");
            if secondary == "dynamic-dispatch" {
                let binding = serde_yaml::from_str("spec: rms/implementation/v0.1\narchitecture:\n  authority_bindings:\n  - {authority: device, safe_facade: 'Executor.swift#execute(using:Operation)'}\n  - {authority: dynamic-dispatch, safe_facade: 'Executor.swift#execute(using:Operation)'}\nsemantic_functions:\n- {id: subject, kind: effect-executor, symbol: 'Executor.swift#execute(using:Operation)', purity: effectful}\n").unwrap();
                assert!(crate::binding_migration::plan(&binding, &result, "v0.2").is_ok());
            }
        }
        let result = analyze(AnalysisInput {
            binding: "rust".into(), source_digest: "source".into(), tool_digest: "tool".into(),
            sources: BTreeMap::from([("src/io.rs".into(), "fn execute() { reqwest::get(\"url\"); }".into())]),
            semantic_functions: vec![expectation("execute", "effectful", &["filesystem"])],
            authority_facades: vec![AuthorityFacade { authority: "filesystem".into(), symbol: "src/io.rs#execute".into() }],
            trusted_external_calls: BTreeSet::new(),
        });
        assert_eq!(result.result, AnalysisResult::Fail, "raw filesystem must not rename network: {result:#?}");
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

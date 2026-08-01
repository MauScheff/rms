use anyhow::Result;
use serde::Serialize;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::{
    binding_symbol_reference_exists, declared_public_behaviors, declared_required_capabilities,
    discover_module_manifests, display_relative, get_path, get_str, get_string_array,
    load_manifest, source_revision, stable_atlas_id, typed_yaml_sequence, warning,
    DependencyBehaviorBinding, Diagnostic, LoadedManifest, MachineEffectProtocol,
    PublicBehaviorBinding,
};

const GRAPH_SPEC: &str = "rms/semantic-system-graph/v0.1";

#[derive(Clone, Debug, Serialize)]
pub(super) struct SemanticSystemGraph {
    pub(super) spec: &'static str,
    pub(super) source_revision: Option<String>,
    pub(super) nodes: Vec<SemanticGraphNode>,
    pub(super) edges: Vec<SemanticGraphEdge>,
    pub(super) obligations: Vec<SemanticGraphObligation>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct SemanticGraphNode {
    pub(super) id: String,
    pub(super) module_id: String,
    pub(super) kind: String,
    pub(super) label: String,
    pub(super) summary: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) details: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(super) lists: BTreeMap<String, Vec<String>>,
    pub(super) source_refs: Vec<SemanticGraphSourceRef>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct SemanticGraphEdge {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) from: String,
    pub(super) to: String,
    pub(super) label: String,
    pub(super) source_refs: Vec<SemanticGraphSourceRef>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct SemanticGraphObligation {
    pub(super) id: String,
    pub(super) module_id: String,
    pub(super) kind: String,
    pub(super) status: &'static str,
    pub(super) title: String,
    pub(super) detail: String,
    pub(super) source_refs: Vec<SemanticGraphSourceRef>,
}

#[derive(Clone, Debug, Serialize)]
pub(super) struct SemanticGraphSourceRef {
    pub(super) role: String,
    pub(super) path: String,
}

#[derive(Clone)]
struct ModuleProjection {
    manifest: LoadedManifest,
    implementation: Option<LoadedManifest>,
    module_name: String,
    module_id: String,
    module_kind: String,
    shape: Option<String>,
}

impl ModuleProjection {
    fn base(&self) -> &Path {
        self.manifest
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
    }

    fn source(&self, root: &Path) -> SemanticGraphSourceRef {
        source_ref(root, &self.manifest.path, "module-manifest")
    }
}

pub(super) fn build_semantic_system_graph(root: &Path) -> Result<SemanticSystemGraph> {
    let mut manifests = discover_module_manifests(root)?;
    manifests.sort_by(|left, right| {
        get_str(&left.value, &["module", "name"]).cmp(&get_str(&right.value, &["module", "name"]))
    });
    let modules = manifests
        .into_iter()
        .map(module_projection)
        .collect::<Vec<_>>();
    let module_ids = modules
        .iter()
        .map(|module| (module.module_name.clone(), module.module_id.clone()))
        .collect::<BTreeMap<_, _>>();
    let provider_capabilities = modules
        .iter()
        .flat_map(|module| {
            declared_public_behaviors(&module.manifest.value)
                .into_iter()
                .filter(|((kind, _), _)| matches!(kind.as_str(), "capability" | "command"))
                .map(|((_, capability), contract)| {
                    ((module.module_name.clone(), capability), contract)
                })
                .collect::<Vec<_>>()
        })
        .collect::<BTreeMap<_, _>>();

    let mut graph = SemanticSystemGraph {
        spec: GRAPH_SPEC,
        source_revision: source_revision(root),
        nodes: Vec::new(),
        edges: Vec::new(),
        obligations: Vec::new(),
    };
    let mut node_ids = BTreeSet::new();
    let mut edge_ids = BTreeSet::new();
    for module in &modules {
        project_module(
            root,
            module,
            &module_ids,
            &provider_capabilities,
            &mut graph,
            &mut node_ids,
            &mut edge_ids,
        );
    }
    graph.nodes.sort_by(|left, right| left.id.cmp(&right.id));
    graph.edges.sort_by(|left, right| left.id.cmp(&right.id));
    graph
        .obligations
        .sort_by(|left, right| left.id.cmp(&right.id));
    Ok(graph)
}

pub(super) fn semantic_system_graph_diagnostics(root: &Path) -> Result<Vec<Diagnostic>> {
    let graph = build_semantic_system_graph(root)?;
    Ok(graph
        .obligations
        .iter()
        .filter_map(|obligation| {
            let check = match (obligation.kind.as_str(), obligation.status) {
                ("public-binding", "required-gap") => "semantic.public-binding-missing",
                ("public-binding", "unresolved-link") => "semantic.public-binding-invalid",
                ("public-reachability", "unresolved-link") => "semantic.public-input-unreachable",
                ("dependency-binding", "required-gap" | "unresolved-link") => {
                    "semantic.required-capability-binding-missing"
                }
                ("effect-owner", "required-gap" | "unresolved-link") => {
                    "semantic.effect-owner-missing"
                }
                ("invariant-proof-chain", "required-gap" | "unresolved-link") => {
                    "semantic.invariant-proof-chain-incomplete"
                }
                ("public-proof-chain", "required-gap") => "semantic.public-proof-chain-incomplete",
                ("trace-case", "required-gap" | "unresolved-link") => {
                    "semantic.trace-case-unrepresented"
                }
                ("module-link", "required-gap" | "unresolved-link") => {
                    "semantic.module-link-unresolved"
                }
                _ => return None,
            };
            let path = obligation
                .source_refs
                .first()
                .map(|source| PathBuf::from(&source.path))
                .unwrap_or_else(|| root.to_path_buf());
            Some(warning(check, &path, obligation.detail.clone()))
        })
        .collect())
}

fn module_projection(manifest: LoadedManifest) -> ModuleProjection {
    let base = manifest.path.parent().unwrap_or_else(|| Path::new("."));
    let implementation = base
        .join("implementation.yaml")
        .is_file()
        .then(|| load_manifest(&base.join("implementation.yaml")).ok())
        .flatten();
    let module_name = get_str(&manifest.value, &["module", "name"])
        .unwrap_or("module")
        .to_string();
    let module_kind = get_str(&manifest.value, &["module", "kind"])
        .unwrap_or("module")
        .to_string();
    let shape = implementation
        .as_ref()
        .and_then(|implementation| get_str(&implementation.value, &["architecture", "shape"]))
        .or_else(|| get_str(&manifest.value, &["x-scaffold", "shape"]))
        .map(ToString::to_string);
    ModuleProjection {
        module_id: stable_atlas_id("module", &module_name),
        manifest,
        implementation,
        module_name,
        module_kind,
        shape,
    }
}

fn project_module(
    root: &Path,
    module: &ModuleProjection,
    module_ids: &BTreeMap<String, String>,
    provider_capabilities: &BTreeMap<(String, String), String>,
    graph: &mut SemanticSystemGraph,
    node_ids: &mut BTreeSet<String>,
    edge_ids: &mut BTreeSet<String>,
) {
    let module_source = module.source(root);
    push_node(
        graph,
        node_ids,
        SemanticGraphNode {
            id: module.module_id.clone(),
            module_id: module.module_id.clone(),
            kind: "module".to_string(),
            label: module.module_name.clone(),
            summary: get_str(&module.manifest.value, &["module", "purpose"])
                .unwrap_or("RMS module")
                .to_string(),
            details: BTreeMap::from([
                ("kind".to_string(), module.module_kind.clone()),
                (
                    "shape".to_string(),
                    module
                        .shape
                        .clone()
                        .unwrap_or_else(|| "semantic-only".to_string()),
                ),
            ]),
            lists: BTreeMap::new(),
            source_refs: vec![module_source.clone()],
        },
    );

    let public_behaviors = declared_public_behaviors(&module.manifest.value);
    let required_capabilities = declared_required_capabilities(&module.manifest.value);
    project_module_topology(root, module, module_ids, &public_behaviors, graph, edge_ids);
    let invariant_ids =
        project_module_semantics(root, module, graph, node_ids, edge_ids, &module_source);
    let Some(implementation) = module.implementation.as_ref() else {
        let no_trace_cases = BTreeSet::new();
        project_shape_obligations(
            root,
            module,
            graph,
            &public_behaviors,
            &required_capabilities,
            false,
            &[],
            &[],
            &no_trace_cases,
        );
        return;
    };

    let implementation_source = source_ref(root, &implementation.path, "implementation-binding");
    let implementation_id = graph_id(&module.module_name, "implementation", "binding");
    push_node(
        graph,
        node_ids,
        SemanticGraphNode {
            id: implementation_id.clone(),
            module_id: module.module_id.clone(),
            kind: "implementation".to_string(),
            label: get_str(&implementation.value, &["binding"])
                .unwrap_or("binding")
                .to_string(),
            summary: "Binding realization of the module's canonical semantics.".to_string(),
            details: BTreeMap::from([(
                "binding".to_string(),
                get_str(&implementation.value, &["binding"])
                    .unwrap_or("binding")
                    .to_string(),
            )]),
            lists: BTreeMap::new(),
            source_refs: vec![implementation_source.clone()],
        },
    );
    push_edge(
        graph,
        edge_ids,
        "realizes",
        &implementation_id,
        &module.module_id,
        "realizes module semantics",
        vec![implementation_source.clone()],
    );

    let function_ids = project_implementation_semantics(
        root,
        module,
        implementation,
        graph,
        node_ids,
        edge_ids,
        &implementation_id,
    );
    let public_bindings = typed_yaml_sequence::<PublicBehaviorBinding>(
        &implementation.value,
        &["architecture", "public_behavior_bindings"],
    );
    project_public_bindings(
        root,
        module,
        implementation,
        graph,
        node_ids,
        edge_ids,
        &public_behaviors,
        &public_bindings,
        &function_ids,
    );
    let dependency_bindings = typed_yaml_sequence::<DependencyBehaviorBinding>(
        &implementation.value,
        &["architecture", "dependency_behavior_bindings"],
    );
    project_dependency_bindings(
        root,
        module,
        graph,
        node_ids,
        edge_ids,
        module_ids,
        provider_capabilities,
        &required_capabilities,
        &dependency_bindings,
    );
    project_trace_records(root, module, implementation, graph, node_ids, edge_ids);
    project_invariant_closure(root, module, implementation, graph, &invariant_ids);
    let transition_cases = machine_transition_cases(&implementation.value);
    let trace_cases = recorded_trace_cases(module.base(), &implementation.value);
    project_trace_case_closure(root, module, graph, &transition_cases, &trace_cases);
    let machine_effects = get_string_array(
        &implementation.value,
        &["architecture", "machine", "effects"],
    );
    let protocols = typed_yaml_sequence::<MachineEffectProtocol>(
        &implementation.value,
        &["architecture", "machine", "effect_protocols"],
    );
    project_effect_closure(root, module, graph, &machine_effects, &protocols);
    project_shape_obligations(
        root,
        module,
        graph,
        &public_behaviors,
        &required_capabilities,
        true,
        &machine_effects,
        &transition_cases,
        &trace_cases,
    );
}

fn project_trace_records(
    root: &Path,
    module: &ModuleProjection,
    implementation: &LoadedManifest,
    graph: &mut SemanticSystemGraph,
    node_ids: &mut BTreeSet<String>,
    edge_ids: &mut BTreeSet<String>,
) {
    let Some(producers) = get_path(
        &implementation.value,
        &["architecture", "trace", "producers"],
    )
    .and_then(YamlValue::as_sequence) else {
        return;
    };
    for producer in producers {
        let Some(bundle) = get_str(producer, &["bundle"]) else {
            continue;
        };
        let producer_id = get_str(producer, &["id"]).unwrap_or(bundle);
        let bundle_path = module.base().join(bundle);
        let Ok(contents) = fs::read_to_string(&bundle_path) else {
            continue;
        };
        let Ok(document) = serde_yaml::from_str::<YamlValue>(&contents) else {
            continue;
        };
        let artifact_spec = get_str(&document, &["spec"]).unwrap_or("");
        let probe_trace = match artifact_spec {
            "rms/probe-system-trace/v0.1" => Some(&document),
            "rms/probe-counterexample/v0.1" => get_path(&document, &["trace"]),
            _ => None,
        };
        let source = source_ref(root, &bundle_path, "trace-bundle");
        let bundle_id = graph_id(&module.module_name, "trace-bundle", producer_id);
        let mut bundle_details = BTreeMap::from([
            ("bundle".to_string(), bundle.to_string()),
            (
                "profile".to_string(),
                get_str(producer, &["profile"]).unwrap_or("").to_string(),
            ),
            (
                "command".to_string(),
                get_str(producer, &["command"]).unwrap_or("").to_string(),
            ),
            (
                "runner".to_string(),
                get_str(producer, &["runner"]).unwrap_or("").to_string(),
            ),
        ]);
        if let Some(trace) = probe_trace {
            bundle_details.insert(
                "result".to_string(),
                get_str(trace, &["result"]).unwrap_or("").to_string(),
            );
            bundle_details.insert(
                "assembly_digest".to_string(),
                get_str(trace, &["assembly_digest"])
                    .unwrap_or("")
                    .to_string(),
            );
            bundle_details.insert(
                "states_explored".to_string(),
                yaml_text(get_path(trace, &["coverage", "states"])),
            );
            bundle_details.insert(
                "schedules_completed".to_string(),
                yaml_text(get_path(trace, &["coverage", "schedules"])),
            );
        }
        push_node(
            graph,
            node_ids,
            SemanticGraphNode {
                id: bundle_id.clone(),
                module_id: module.module_id.clone(),
                kind: "trace-bundle".to_string(),
                label: producer_id.to_string(),
                summary: if probe_trace.is_some() {
                    format!("Deterministic multi-module probe timeline from {bundle}.")
                } else {
                    format!("Execution-derived transition records from {bundle}.")
                },
                details: bundle_details,
                lists: BTreeMap::new(),
                source_refs: vec![source.clone()],
            },
        );
        if let Some(trace) = probe_trace {
            project_probe_timeline(
                module,
                graph,
                node_ids,
                edge_ids,
                producer_id,
                &bundle_id,
                trace,
                &source,
            );
            continue;
        }
        let Some(records) = get_path(&document, &["records"]).and_then(YamlValue::as_sequence)
        else {
            continue;
        };
        for (index, record) in records.iter().enumerate() {
            let case = get_str(record, &["source", "branch"])
                .or_else(|| get_str(record, &["source", "case"]))
                .unwrap_or("unclassified");
            let record_id = graph_id(
                &module.module_name,
                "trace-record",
                &format!("{producer_id}-{index}-{case}"),
            );
            let scenario_start = get_path(record, &["scenario_start"])
                .and_then(YamlValue::as_bool)
                .unwrap_or(false);
            push_node(
                graph,
                node_ids,
                SemanticGraphNode {
                    id: record_id.clone(),
                    module_id: module.module_id.clone(),
                    kind: "trace-record".to_string(),
                    label: format!("{} · {}", index + 1, case),
                    summary: get_str(record, &["input"])
                        .unwrap_or("Recorded machine input")
                        .to_string(),
                    details: BTreeMap::from([
                        ("case".to_string(), case.to_string()),
                        (
                            "input".to_string(),
                            get_str(record, &["input"]).unwrap_or("").to_string(),
                        ),
                        (
                            "state_before".to_string(),
                            get_str(record, &["state_before"]).unwrap_or("").to_string(),
                        ),
                        (
                            "state_after".to_string(),
                            get_str(record, &["state_after"]).unwrap_or("").to_string(),
                        ),
                        (
                            "reply".to_string(),
                            get_str(record, &["output", "reply"])
                                .unwrap_or("")
                                .to_string(),
                        ),
                        (
                            "rejection".to_string(),
                            get_str(record, &["output", "rejection"])
                                .unwrap_or("")
                                .to_string(),
                        ),
                        (
                            "source_file".to_string(),
                            get_str(record, &["source", "file"])
                                .unwrap_or("")
                                .to_string(),
                        ),
                        (
                            "source_function".to_string(),
                            get_str(record, &["source", "function"])
                                .unwrap_or("")
                                .to_string(),
                        ),
                        ("scenario_start".to_string(), scenario_start.to_string()),
                    ]),
                    lists: BTreeMap::from([
                        (
                            "events".to_string(),
                            get_string_array(record, &["output", "events"]),
                        ),
                        (
                            "commands".to_string(),
                            get_string_array(record, &["output", "commands"]),
                        ),
                        (
                            "effects".to_string(),
                            get_string_array(record, &["output", "effects"]),
                        ),
                    ]),
                    source_refs: vec![source.clone()],
                },
            );
            push_edge(
                graph,
                edge_ids,
                "contains",
                &bundle_id,
                &record_id,
                "transition record",
                vec![source.clone()],
            );
            if case != "unclassified" {
                push_edge(
                    graph,
                    edge_ids,
                    "records",
                    &record_id,
                    &graph_id(&module.module_name, "transition-case", case),
                    "records transition case",
                    vec![source.clone()],
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn project_probe_timeline(
    module: &ModuleProjection,
    graph: &mut SemanticSystemGraph,
    node_ids: &mut BTreeSet<String>,
    edge_ids: &mut BTreeSet<String>,
    producer_id: &str,
    bundle_id: &str,
    trace: &YamlValue,
    source: &SemanticGraphSourceRef,
) {
    let Some(entries) = get_path(trace, &["timeline"]).and_then(YamlValue::as_sequence) else {
        return;
    };
    for (index, entry) in entries.iter().enumerate() {
        let case = get_str(entry, &["transition_case"]).unwrap_or("");
        let action = get_str(entry, &["action"]).unwrap_or("step");
        let target = get_str(entry, &["target"]).unwrap_or("unknown");
        let step = yaml_text(get_path(entry, &["step"]));
        let record_id = graph_id(
            &module.module_name,
            "trace-record",
            &format!("{producer_id}-{index}-{target}-{case}-{action}"),
        );
        push_node(
            graph,
            node_ids,
            SemanticGraphNode {
                id: record_id.clone(),
                module_id: module.module_id.clone(),
                kind: "trace-record".to_string(),
                label: format!(
                    "{} · {} · {}",
                    if step.is_empty() {
                        (index + 1).to_string()
                    } else {
                        step
                    },
                    target,
                    if case.is_empty() { action } else { case }
                ),
                summary: yaml_text(get_path(entry, &["input"])),
                details: BTreeMap::from([
                    ("case".to_string(), case.to_string()),
                    ("action".to_string(), action.to_string()),
                    ("time".to_string(), yaml_text(get_path(entry, &["time"]))),
                    (
                        "route".to_string(),
                        get_str(entry, &["route"]).unwrap_or("").to_string(),
                    ),
                    (
                        "source".to_string(),
                        get_str(entry, &["source"]).unwrap_or("").to_string(),
                    ),
                    ("target".to_string(), target.to_string()),
                    (
                        "correlation".to_string(),
                        get_str(entry, &["correlation_id"])
                            .unwrap_or("")
                            .to_string(),
                    ),
                    (
                        "causation".to_string(),
                        get_str(entry, &["causation_id"]).unwrap_or("").to_string(),
                    ),
                    (
                        "idempotency".to_string(),
                        get_str(entry, &["idempotency_key"])
                            .unwrap_or("")
                            .to_string(),
                    ),
                    (
                        "attempt".to_string(),
                        yaml_text(get_path(entry, &["attempt"])),
                    ),
                    (
                        "state_before".to_string(),
                        yaml_text(get_path(entry, &["state_before"])),
                    ),
                    (
                        "state_after".to_string(),
                        yaml_text(get_path(entry, &["state_after"])),
                    ),
                    (
                        "source_file".to_string(),
                        get_str(entry, &["source_file"]).unwrap_or("").to_string(),
                    ),
                    (
                        "source_function".to_string(),
                        get_str(entry, &["source_function"])
                            .unwrap_or("")
                            .to_string(),
                    ),
                ]),
                lists: BTreeMap::new(),
                source_refs: vec![source.clone()],
            },
        );
        push_edge(
            graph,
            edge_ids,
            "contains",
            bundle_id,
            &record_id,
            "probe timeline step",
            vec![source.clone()],
        );
        if !case.is_empty() {
            push_edge(
                graph,
                edge_ids,
                "records",
                &record_id,
                &graph_id(&module.module_name, "transition-case", case),
                "records transition case",
                vec![source.clone()],
            );
        }
    }
}

fn yaml_text(value: Option<&YamlValue>) -> String {
    value
        .and_then(|value| serde_json::to_value(value).ok())
        .and_then(|value| match value {
            serde_json::Value::Null => None,
            serde_json::Value::String(value) => Some(value),
            other => serde_json::to_string(&other).ok(),
        })
        .unwrap_or_default()
}

fn project_module_topology(
    root: &Path,
    module: &ModuleProjection,
    module_ids: &BTreeMap<String, String>,
    public_behaviors: &BTreeMap<(String, String), String>,
    graph: &mut SemanticSystemGraph,
    edge_ids: &mut BTreeSet<String>,
) {
    let source = vec![module.source(root)];
    for (path, edge_kind, label) in [
        (["composition", "contains"], "contains", "contains module"),
        (
            ["requires", "modules"],
            "requires-module",
            "requires module",
        ),
    ] {
        let Some(items) = get_path(&module.manifest.value, &path).and_then(YamlValue::as_sequence)
        else {
            continue;
        };
        for item in items {
            let Some(name) = item.as_str().or_else(|| get_str(item, &["name"])) else {
                continue;
            };
            if let Some(target) = module_ids.get(name) {
                push_edge(
                    graph,
                    edge_ids,
                    edge_kind,
                    &module.module_id,
                    target,
                    label,
                    source.clone(),
                );
            } else {
                push_obligation(
                    graph,
                    module,
                    "module-link",
                    "unresolved-link",
                    format!("Resolve {edge_kind} `{name}`"),
                    format!(
                        "module `{}` references undiscovered module `{name}`",
                        module.module_name
                    ),
                    source.clone(),
                );
            }
        }
    }
    let Some(exports) = get_path(&module.manifest.value, &["composition", "exports"])
        .and_then(YamlValue::as_sequence)
    else {
        return;
    };
    for export in exports {
        let (Some(group), Some(name), Some(provider)) = (
            get_str(export, &["group"]),
            get_str(export, &["name"]),
            get_str(export, &["from"]),
        ) else {
            continue;
        };
        let public_kind = public_behavior_kind(group);
        let public_id = graph_id(&module.module_name, &format!("public-{public_kind}"), name);
        let resolves_public =
            public_behaviors.contains_key(&(public_kind.to_string(), name.to_string()));
        if let Some(provider_id) = module_ids.get(provider).filter(|_| resolves_public) {
            push_edge(
                graph,
                edge_ids,
                "exports",
                provider_id,
                &public_id,
                &format!("exports {public_kind}"),
                source.clone(),
            );
        } else {
            push_obligation(
                graph,
                module,
                "module-link",
                "unresolved-link",
                format!("Resolve export `{name}`"),
                format!(
                    "composite export `{name}` does not resolve both provider `{provider}` and public {public_kind} declaration"
                ),
                source.clone(),
            );
        }
    }
}

fn public_behavior_kind(group: &str) -> &str {
    match group {
        "commands" => "command",
        "queries" => "query",
        "events" => "event",
        "capabilities" => "capability",
        other => other,
    }
}

fn project_module_semantics(
    root: &Path,
    module: &ModuleProjection,
    graph: &mut SemanticSystemGraph,
    node_ids: &mut BTreeSet<String>,
    edge_ids: &mut BTreeSet<String>,
    module_source: &SemanticGraphSourceRef,
) -> Vec<String> {
    let mut invariants = Vec::new();
    for (kind, section) in [
        ("command", "commands"),
        ("query", "queries"),
        ("event", "events"),
        ("capability", "capabilities"),
    ] {
        let Some(items) = get_path(&module.manifest.value, &["provides", section])
            .and_then(YamlValue::as_sequence)
        else {
            continue;
        };
        for item in items {
            let Some(name) = get_str(item, &["name"]) else {
                continue;
            };
            let id = graph_id(&module.module_name, &format!("public-{kind}"), name);
            let mut refs = vec![module_source.clone()];
            if let Some(contract) = get_str(item, &["contract"]) {
                refs.push(source_ref(root, &module.base().join(contract), "contract"));
            }
            push_node(
                graph,
                node_ids,
                SemanticGraphNode {
                    id: id.clone(),
                    module_id: module.module_id.clone(),
                    kind: format!("public-{kind}"),
                    label: name.to_string(),
                    summary: format!("Public {kind} declared by {}.", module.module_name),
                    details: BTreeMap::from([
                        ("public_kind".to_string(), kind.to_string()),
                        (
                            "contract".to_string(),
                            get_str(item, &["contract"]).unwrap_or("").to_string(),
                        ),
                    ]),
                    lists: BTreeMap::new(),
                    source_refs: refs.clone(),
                },
            );
            push_edge(
                graph,
                edge_ids,
                "provides",
                &module.module_id,
                &id,
                &format!("provides {kind}"),
                refs,
            );
        }
    }
    if let Some(items) = get_path(&module.manifest.value, &["requires", "capabilities"])
        .and_then(YamlValue::as_sequence)
    {
        for item in items {
            let Some(name) = get_str(item, &["name"]) else {
                continue;
            };
            let id = graph_id(&module.module_name, "required-capability", name);
            let mut refs = vec![module_source.clone()];
            if let Some(contract) = get_str(item, &["contract"]) {
                refs.push(source_ref(
                    root,
                    &module.base().join(contract),
                    "required-contract",
                ));
            }
            push_node(
                graph,
                node_ids,
                SemanticGraphNode {
                    id: id.clone(),
                    module_id: module.module_id.clone(),
                    kind: "required-capability".to_string(),
                    label: name.to_string(),
                    summary: "Required behavior owned outside this module.".to_string(),
                    details: BTreeMap::from([(
                        "contract".to_string(),
                        get_str(item, &["contract"]).unwrap_or("").to_string(),
                    )]),
                    lists: BTreeMap::new(),
                    source_refs: refs.clone(),
                },
            );
            push_edge(
                graph,
                edge_ids,
                "requires",
                &module.module_id,
                &id,
                "requires capability",
                refs,
            );
        }
    }
    if let Some(items) =
        get_path(&module.manifest.value, &["invariants"]).and_then(YamlValue::as_sequence)
    {
        for item in items {
            let Some(id_value) = get_str(item, &["id"]) else {
                continue;
            };
            invariants.push(id_value.to_string());
            let id = graph_id(&module.module_name, "invariant", id_value);
            let mut refs = vec![module_source.clone()];
            if let Some(evidence) = get_str(item, &["verified_by"]) {
                refs.push(source_ref(root, &module.base().join(evidence), "evidence"));
            }
            push_node(
                graph,
                node_ids,
                SemanticGraphNode {
                    id: id.clone(),
                    module_id: module.module_id.clone(),
                    kind: "invariant".to_string(),
                    label: id_value.to_string(),
                    summary: get_str(item, &["statement"])
                        .unwrap_or("Declared invariant")
                        .to_string(),
                    details: BTreeMap::from([
                        (
                            "authority".to_string(),
                            get_str(item, &["authority"]).unwrap_or("").to_string(),
                        ),
                        (
                            "enforced_by".to_string(),
                            get_str(item, &["enforced_by"]).unwrap_or("").to_string(),
                        ),
                    ]),
                    lists: BTreeMap::new(),
                    source_refs: refs.clone(),
                },
            );
            push_edge(
                graph,
                edge_ids,
                "constrains",
                &id,
                &module.module_id,
                "constrains module",
                refs,
            );
            if let Some(evidence) = get_str(item, &["verified_by"]) {
                push_edge(
                    graph,
                    edge_ids,
                    "evidences",
                    &graph_id(&module.module_name, "evidence", evidence),
                    &id,
                    "proves invariant",
                    vec![source_ref(root, &module.base().join(evidence), "evidence")],
                );
            }
        }
    }
    project_evidence_nodes(root, module, graph, node_ids, edge_ids);
    invariants
}

fn project_evidence_nodes(
    root: &Path,
    module: &ModuleProjection,
    graph: &mut SemanticSystemGraph,
    node_ids: &mut BTreeSet<String>,
    edge_ids: &mut BTreeSet<String>,
) {
    let Some(verification) =
        get_path(&module.manifest.value, &["verification"]).and_then(YamlValue::as_mapping)
    else {
        return;
    };
    for (category, values) in verification {
        let category = category.as_str().unwrap_or("evidence");
        let Some(values) = values.as_sequence() else {
            continue;
        };
        for path in values.iter().filter_map(YamlValue::as_str) {
            let id = graph_id(&module.module_name, "evidence", path);
            let evidence_ref = source_ref(root, &module.base().join(path), category);
            push_node(
                graph,
                node_ids,
                SemanticGraphNode {
                    id: id.clone(),
                    module_id: module.module_id.clone(),
                    kind: "evidence".to_string(),
                    label: path.to_string(),
                    summary: format!("{category} evidence"),
                    details: BTreeMap::from([
                        ("category".to_string(), category.to_string()),
                        ("path".to_string(), path.to_string()),
                    ]),
                    lists: BTreeMap::new(),
                    source_refs: vec![evidence_ref.clone()],
                },
            );
            push_edge(
                graph,
                edge_ids,
                "evidences",
                &id,
                &module.module_id,
                category,
                vec![evidence_ref],
            );
        }
    }
}

fn project_implementation_semantics(
    root: &Path,
    module: &ModuleProjection,
    implementation: &LoadedManifest,
    graph: &mut SemanticSystemGraph,
    node_ids: &mut BTreeSet<String>,
    edge_ids: &mut BTreeSet<String>,
    implementation_id: &str,
) -> BTreeMap<String, String> {
    let source = source_ref(root, &implementation.path, "implementation-binding");
    let mut functions = BTreeMap::new();
    if let Some(machine) = get_path(&implementation.value, &["architecture", "machine"])
        .and_then(YamlValue::as_mapping)
    {
        let machine_name = machine
            .get(YamlValue::String("name".to_string()))
            .and_then(YamlValue::as_str)
            .unwrap_or("machine");
        let machine_id = graph_id(&module.module_name, "machine", machine_name);
        push_node(
            graph,
            node_ids,
            SemanticGraphNode {
                id: machine_id.clone(),
                module_id: module.module_id.clone(),
                kind: "machine".to_string(),
                label: machine_name.to_string(),
                summary: format!(
                    "{} machine",
                    get_str(&implementation.value, &["architecture", "machine", "mode"])
                        .unwrap_or("declared")
                ),
                details: BTreeMap::from([
                    (
                        "mode".to_string(),
                        get_str(&implementation.value, &["architecture", "machine", "mode"])
                            .unwrap_or("declared")
                            .to_string(),
                    ),
                    (
                        "initial_state".to_string(),
                        get_str(
                            &implementation.value,
                            &["architecture", "machine", "initial_state"],
                        )
                        .unwrap_or("")
                        .to_string(),
                    ),
                    (
                        "transition_signature".to_string(),
                        get_str(
                            &implementation.value,
                            &["architecture", "machine", "transition_signature"],
                        )
                        .unwrap_or("")
                        .to_string(),
                    ),
                ]),
                lists: BTreeMap::from([(
                    "terminal_states".to_string(),
                    get_string_array(
                        &implementation.value,
                        &["architecture", "machine", "terminal_states"],
                    ),
                )]),
                source_refs: vec![source.clone()],
            },
        );
        push_edge(
            graph,
            edge_ids,
            "implements",
            implementation_id,
            &machine_id,
            "implements machine",
            vec![source.clone()],
        );
        for (field, kind) in [
            ("states", "state"),
            ("commands", "command"),
            ("observed_events", "observed-event"),
            ("events", "event"),
            ("effects", "effect"),
            ("effect_results", "effect-result"),
            ("replies", "reply"),
            ("rejections", "rejection"),
        ] {
            for value in
                get_string_array(&implementation.value, &["architecture", "machine", field])
            {
                let id = graph_id(&module.module_name, kind, &value);
                push_node(
                    graph,
                    node_ids,
                    SemanticGraphNode {
                        id: id.clone(),
                        module_id: module.module_id.clone(),
                        kind: kind.to_string(),
                        label: value,
                        summary: format!("Declared machine {kind}."),
                        details: BTreeMap::from([("category".to_string(), field.to_string())]),
                        lists: BTreeMap::new(),
                        source_refs: vec![source.clone()],
                    },
                );
                push_edge(
                    graph,
                    edge_ids,
                    "contains",
                    &machine_id,
                    &id,
                    kind,
                    vec![source.clone()],
                );
            }
        }
        if let Some(transitions) = machine
            .get(YamlValue::String("transitions".to_string()))
            .and_then(YamlValue::as_sequence)
        {
            for transition in transitions {
                let case = get_str(transition, &["case"]).unwrap_or("transition");
                let id = graph_id(&module.module_name, "transition-case", case);
                let from = get_str(transition, &["from"]).unwrap_or("?");
                let on = get_str(transition, &["on"]).unwrap_or("?");
                let to = get_str(transition, &["to"]).unwrap_or("?");
                push_node(
                    graph,
                    node_ids,
                    SemanticGraphNode {
                        id: id.clone(),
                        module_id: module.module_id.clone(),
                        kind: "transition-case".to_string(),
                        label: case.to_string(),
                        summary: format!("{from} --{on}--> {to}"),
                        details: BTreeMap::from([
                            ("from".to_string(), from.to_string()),
                            ("on".to_string(), on.to_string()),
                            ("to".to_string(), to.to_string()),
                            (
                                "reply".to_string(),
                                get_str(transition, &["reply"]).unwrap_or("").to_string(),
                            ),
                            (
                                "rejection".to_string(),
                                get_str(transition, &["rejection"])
                                    .unwrap_or("")
                                    .to_string(),
                            ),
                        ]),
                        lists: BTreeMap::from([
                            (
                                "events".to_string(),
                                get_string_array(transition, &["events"]),
                            ),
                            (
                                "commands".to_string(),
                                get_string_array(transition, &["commands"]),
                            ),
                            (
                                "effects".to_string(),
                                get_string_array(transition, &["effects"]),
                            ),
                        ]),
                        source_refs: vec![source.clone()],
                    },
                );
                push_edge(
                    graph,
                    edge_ids,
                    "contains",
                    &machine_id,
                    &id,
                    "transition case",
                    vec![source.clone()],
                );
                for (edge_kind, target_kind, values) in [
                    (
                        "accepts",
                        input_kind(&implementation.value, on),
                        vec![on.to_string()],
                    ),
                    ("transitions-to", "state", vec![to.to_string()]),
                    ("emits", "event", get_string_array(transition, &["events"])),
                    (
                        "emits",
                        "command",
                        get_string_array(transition, &["commands"]),
                    ),
                    (
                        "emits",
                        "effect",
                        get_string_array(transition, &["effects"]),
                    ),
                    (
                        "returns",
                        "reply",
                        get_str(transition, &["reply"])
                            .map(ToString::to_string)
                            .into_iter()
                            .collect(),
                    ),
                    (
                        "rejects",
                        "rejection",
                        get_str(transition, &["rejection"])
                            .map(ToString::to_string)
                            .into_iter()
                            .collect(),
                    ),
                ] {
                    for value in values {
                        let target = graph_id(&module.module_name, target_kind, &value);
                        push_edge(
                            graph,
                            edge_ids,
                            edge_kind,
                            &id,
                            &target,
                            edge_kind,
                            vec![source.clone()],
                        );
                    }
                }
            }
        }
    }
    if let Some(items) =
        get_path(&implementation.value, &["semantic_functions"]).and_then(YamlValue::as_sequence)
    {
        for item in items {
            let Some(function_id) = get_str(item, &["id"]) else {
                continue;
            };
            let id = graph_id(&module.module_name, "semantic-function", function_id);
            functions.insert(function_id.to_string(), id.clone());
            let refs = get_str(item, &["symbol"])
                .and_then(|symbol| symbol.rsplit_once('#').map(|(path, _)| path))
                .map(|path| vec![source_ref(root, &module.base().join(path), "source-symbol")])
                .unwrap_or_else(|| vec![source.clone()]);
            push_node(
                graph,
                node_ids,
                SemanticGraphNode {
                    id: id.clone(),
                    module_id: module.module_id.clone(),
                    kind: "semantic-function".to_string(),
                    label: function_id.to_string(),
                    summary: get_str(item, &["symbol"])
                        .unwrap_or("binding symbol")
                        .to_string(),
                    details: BTreeMap::from([
                        (
                            "symbol".to_string(),
                            get_str(item, &["symbol"]).unwrap_or("").to_string(),
                        ),
                        (
                            "function_kind".to_string(),
                            get_str(item, &["kind"]).unwrap_or("").to_string(),
                        ),
                        (
                            "purity".to_string(),
                            get_str(item, &["purity"]).unwrap_or("").to_string(),
                        ),
                    ]),
                    lists: BTreeMap::from([
                        (
                            "contracts".to_string(),
                            get_string_array(item, &["discharges", "contracts"]),
                        ),
                        (
                            "invariants".to_string(),
                            get_string_array(item, &["discharges", "invariants"]),
                        ),
                    ]),
                    source_refs: refs.clone(),
                },
            );
            push_edge(
                graph,
                edge_ids,
                "realizes",
                &id,
                implementation_id,
                "declared semantic function",
                refs,
            );
            for invariant in get_string_array(item, &["discharges", "invariants"]) {
                push_edge(
                    graph,
                    edge_ids,
                    "discharges",
                    &id,
                    &graph_id(&module.module_name, "invariant", &invariant),
                    "discharges invariant",
                    vec![source.clone()],
                );
            }
            for contract in get_string_array(item, &["discharges", "contracts"]) {
                for ((kind, name), public_contract) in
                    declared_public_behaviors(&module.manifest.value)
                {
                    if public_contract != contract {
                        continue;
                    }
                    push_edge(
                        graph,
                        edge_ids,
                        "discharges",
                        &id,
                        &graph_id(&module.module_name, &format!("public-{kind}"), &name),
                        "discharges public contract",
                        vec![source.clone()],
                    );
                }
            }
        }
    }
    if let Some(roles) =
        get_path(&implementation.value, &["architecture", "roles"]).and_then(YamlValue::as_mapping)
    {
        for (kind, values) in roles {
            let kind = kind.as_str().unwrap_or("role");
            for path in values
                .as_sequence()
                .into_iter()
                .flatten()
                .filter_map(YamlValue::as_str)
            {
                let id = graph_id(&module.module_name, "role", &format!("{kind}:{path}"));
                let role_ref = source_ref(root, &module.base().join(path), kind);
                push_node(
                    graph,
                    node_ids,
                    SemanticGraphNode {
                        id: id.clone(),
                        module_id: module.module_id.clone(),
                        kind: "role".to_string(),
                        label: kind.to_string(),
                        summary: path.to_string(),
                        details: BTreeMap::from([
                            ("role".to_string(), kind.to_string()),
                            ("path".to_string(), path.to_string()),
                        ]),
                        lists: BTreeMap::new(),
                        source_refs: vec![role_ref.clone()],
                    },
                );
                push_edge(
                    graph,
                    edge_ids,
                    "contains",
                    implementation_id,
                    &id,
                    kind,
                    vec![role_ref],
                );
            }
        }
    }
    functions
}

#[allow(clippy::too_many_arguments)]
fn project_public_bindings(
    root: &Path,
    module: &ModuleProjection,
    implementation: &LoadedManifest,
    graph: &mut SemanticSystemGraph,
    node_ids: &mut BTreeSet<String>,
    edge_ids: &mut BTreeSet<String>,
    public_behaviors: &BTreeMap<(String, String), String>,
    bindings: &[PublicBehaviorBinding],
    function_ids: &BTreeMap<String, String>,
) {
    let source = source_ref(root, &implementation.path, "public-behavior-binding");
    let semantic_functions = get_path(&implementation.value, &["semantic_functions"])
        .and_then(YamlValue::as_sequence)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for ((kind, name), contract) in public_behaviors {
        let public_id = graph_id(&module.module_name, &format!("public-{kind}"), name);
        let matches = bindings
            .iter()
            .filter(|binding| binding.public_kind == *kind && binding.public_name == *name)
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            push_obligation(
                graph,
                module,
                "public-binding",
                "required-gap",
                format!("Bind public {kind} `{name}`"),
                format!(
                    "implemented public {kind} `{name}` must have exactly one public behavior binding to `{contract}`"
                ),
                vec![module.source(root)],
            );
            continue;
        }
        let binding = matches[0];
        let binding_id = graph_id(&module.module_name, "public-binding", &binding.id);
        push_node(
            graph,
            node_ids,
            SemanticGraphNode {
                id: binding_id.clone(),
                module_id: module.module_id.clone(),
                kind: "public-behavior-binding".to_string(),
                label: binding.id.clone(),
                summary: format!("{kind} `{name}` -> `{}`", binding.semantic_function),
                details: BTreeMap::from([
                    ("public_kind".to_string(), kind.to_string()),
                    ("public_name".to_string(), name.to_string()),
                    ("contract".to_string(), binding.contract.clone()),
                    (
                        "semantic_function".to_string(),
                        binding.semantic_function.clone(),
                    ),
                ]),
                lists: BTreeMap::from([
                    ("machine_inputs".to_string(), binding.machine_inputs.clone()),
                    (
                        "machine_outputs".to_string(),
                        binding.machine_outputs.clone(),
                    ),
                ]),
                source_refs: vec![source.clone()],
            },
        );
        push_edge(
            graph,
            edge_ids,
            "bound-through",
            &public_id,
            &binding_id,
            "bound through",
            vec![source.clone()],
        );
        if let Some(function_id) = function_ids.get(&binding.semantic_function) {
            push_edge(
                graph,
                edge_ids,
                "delegates-to",
                &binding_id,
                function_id,
                "delegates to semantic function",
                vec![source.clone()],
            );
        }
        let classified_inputs = ["commands", "observed_events", "effect_results"]
            .into_iter()
            .flat_map(|field| {
                get_string_array(&implementation.value, &["architecture", "machine", field])
            })
            .collect::<BTreeSet<_>>();
        let classified_outputs = ["events", "effects", "replies", "rejections"]
            .into_iter()
            .flat_map(|field| {
                get_string_array(&implementation.value, &["architecture", "machine", field])
            })
            .collect::<BTreeSet<_>>();
        let semantic_function = semantic_functions.iter().find(|function| {
            get_str(function, &["id"]) == Some(binding.semantic_function.as_str())
        });
        let discharges_contract = semantic_function.is_some_and(|function| {
            get_string_array(function, &["discharges", "contracts"])
                .iter()
                .any(|item| item == contract)
        });
        let function_evidence = semantic_function
            .into_iter()
            .flat_map(|function| {
                get_path(function, &["evidence"])
                    .and_then(YamlValue::as_mapping)
                    .into_iter()
                    .flat_map(|mapping| {
                        mapping.iter().flat_map(|(category, paths)| {
                            let category = category.as_str().unwrap_or("evidence").to_string();
                            paths
                                .as_sequence()
                                .into_iter()
                                .flatten()
                                .filter_map(YamlValue::as_str)
                                .map(move |path| (category.clone(), path.to_string()))
                        })
                    })
            })
            .collect::<Vec<_>>();
        let has_concrete_evidence = function_evidence
            .iter()
            .any(|(_, path)| module.base().join(path).is_file());
        let binding_valid = binding.contract == *contract
            && function_ids.contains_key(&binding.semantic_function)
            && discharges_contract
            && binding
                .machine_inputs
                .iter()
                .all(|input| classified_inputs.contains(input))
            && binding
                .machine_outputs
                .iter()
                .all(|output| classified_outputs.contains(output));
        push_obligation(
            graph,
            module,
            "public-binding",
            if binding_valid {
                "satisfied"
            } else {
                "unresolved-link"
            },
            format!("Bind public {kind} `{name}`"),
            if binding_valid {
                format!(
                    "public {kind} `{name}` is bound to `{}` and `{contract}`",
                    binding.semantic_function
                )
            } else {
                format!(
                    "public behavior binding `{}` does not resolve its contract, semantic function, or machine cases",
                    binding.id
                )
            },
            vec![source.clone()],
        );
        push_obligation(
            graph,
            module,
            "public-proof-chain",
            if !binding_valid {
                "not-applicable"
            } else if has_concrete_evidence {
                "satisfied"
            } else {
                "required-gap"
            },
            format!("Prove public {kind} `{name}`"),
            if !binding_valid {
                format!(
                    "public {kind} `{name}` must first resolve its contract, semantic function, and machine cases"
                )
            } else if has_concrete_evidence {
                format!(
                    "public {kind} `{name}` reaches an exact code symbol and concrete evidence through `{}`",
                    binding.semantic_function
                )
            } else {
                format!(
                    "semantic function `{}` discharges public {kind} `{name}` but has no concrete evidence",
                    binding.semantic_function
                )
            },
            vec![source.clone()],
        );
        if let Some(function_id) = function_ids.get(&binding.semantic_function) {
            for (category, path) in function_evidence {
                let evidence_id = graph_id(&module.module_name, "evidence", &path);
                let evidence_ref = source_ref(root, &module.base().join(&path), &category);
                push_node(
                    graph,
                    node_ids,
                    SemanticGraphNode {
                        id: evidence_id.clone(),
                        module_id: module.module_id.clone(),
                        kind: "evidence".to_string(),
                        label: path.clone(),
                        summary: format!("{category} evidence"),
                        details: BTreeMap::from([
                            ("category".to_string(), category.clone()),
                            ("path".to_string(), path),
                        ]),
                        lists: BTreeMap::new(),
                        source_refs: vec![evidence_ref.clone()],
                    },
                );
                push_edge(
                    graph,
                    edge_ids,
                    "evidences",
                    &evidence_id,
                    function_id,
                    "proves semantic function",
                    vec![evidence_ref],
                );
            }
        }
        for input in &binding.machine_inputs {
            push_edge(
                graph,
                edge_ids,
                "maps-to",
                &binding_id,
                &graph_id(
                    &module.module_name,
                    input_kind(&implementation.value, input),
                    input,
                ),
                "maps to machine input",
                vec![source.clone()],
            );
        }
        let unreachable = matches!(kind.as_str(), "command" | "query")
            && get_path(&implementation.value, &["architecture", "machine"]).is_some()
            && binding.machine_inputs.is_empty();
        push_obligation(
            graph,
            module,
            "public-reachability",
            if unreachable {
                "unresolved-link"
            } else {
                "satisfied"
            },
            format!("Reach public {kind} `{name}`"),
            if unreachable {
                format!("public {kind} `{name}` has no classified machine input")
            } else {
                format!("public {kind} `{name}` is bound through `{}`", binding.id)
            },
            vec![source.clone()],
        );
    }
    for binding in bindings {
        let key = (binding.public_kind.clone(), binding.public_name.clone());
        if public_behaviors.contains_key(&key) {
            continue;
        }
        push_obligation(
            graph,
            module,
            "public-binding",
            "unresolved-link",
            format!("Remove stale public binding `{}`", binding.id),
            format!(
                "public behavior binding `{}` targets undeclared {} `{}`",
                binding.id, binding.public_kind, binding.public_name
            ),
            vec![source.clone()],
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn project_dependency_bindings(
    root: &Path,
    module: &ModuleProjection,
    graph: &mut SemanticSystemGraph,
    node_ids: &mut BTreeSet<String>,
    edge_ids: &mut BTreeSet<String>,
    module_ids: &BTreeMap<String, String>,
    provider_capabilities: &BTreeMap<(String, String), String>,
    required: &BTreeMap<String, Option<String>>,
    bindings: &[DependencyBehaviorBinding],
) {
    let source = module
        .implementation
        .as_ref()
        .map(|implementation| source_ref(root, &implementation.path, "dependency-behavior-binding"))
        .unwrap_or_else(|| module.source(root));
    for (capability, contract) in required {
        let required_id = graph_id(&module.module_name, "required-capability", capability);
        let matches = bindings
            .iter()
            .filter(|binding| binding.capability == *capability)
            .collect::<Vec<_>>();
        let mut status = "satisfied";
        let mut detail =
            format!("required capability `{capability}` has an exact consumer binding");
        if matches.len() != 1 {
            status = "required-gap";
            detail = format!(
                "required capability `{capability}` must have exactly one dependency behavior binding"
            );
        } else {
            let binding = matches[0];
            let binding_id = graph_id(&module.module_name, "dependency-binding", &binding.id);
            push_node(
                graph,
                node_ids,
                SemanticGraphNode {
                    id: binding_id.clone(),
                    module_id: module.module_id.clone(),
                    kind: "dependency-behavior-binding".to_string(),
                    label: binding.id.clone(),
                    summary: format!("{} via {}", binding.capability, binding.consumer),
                    details: BTreeMap::from([
                        ("capability".to_string(), binding.capability.clone()),
                        (
                            "contract".to_string(),
                            binding.contract.clone().unwrap_or_default(),
                        ),
                        ("consumer".to_string(), binding.consumer.clone()),
                        ("resolution".to_string(), binding.resolution.clone()),
                        (
                            "provider_module".to_string(),
                            binding.provider_module.clone().unwrap_or_default(),
                        ),
                        (
                            "provider_contract".to_string(),
                            binding.provider_contract.clone().unwrap_or_default(),
                        ),
                    ]),
                    lists: BTreeMap::new(),
                    source_refs: vec![source.clone()],
                },
            );
            push_edge(
                graph,
                edge_ids,
                "bound-through",
                &required_id,
                &binding_id,
                "bound through consumer port",
                vec![source.clone()],
            );
            let consumer_resolves = module
                .implementation
                .as_ref()
                .is_some_and(|implementation| {
                    binding_symbol_reference_exists(
                        module.base(),
                        implementation,
                        &binding.consumer,
                    )
                });
            if binding.contract.as_ref() != contract.as_ref() {
                status = "unresolved-link";
                detail = format!(
                    "dependency binding `{}` does not match the required contract for `{capability}`",
                    binding.id
                );
            } else if !consumer_resolves {
                status = "unresolved-link";
                detail = format!(
                    "dependency binding `{}` consumer `{}` does not resolve to an implementation symbol",
                    binding.id, binding.consumer
                );
            } else if binding.resolution == "module" {
                let provider_name = binding.provider_module.as_ref();
                let provider = provider_name.and_then(|provider| module_ids.get(provider));
                if let Some(provider) = provider {
                    push_edge(
                        graph,
                        edge_ids,
                        "delegates-to",
                        &binding_id,
                        provider,
                        &format!("requires capability `{capability}`"),
                        vec![source.clone()],
                    );
                    let provider_contract = provider_name.and_then(|provider| {
                        provider_capabilities.get(&(provider.clone(), capability.clone()))
                    });
                    if provider_contract != binding.provider_contract.as_ref() {
                        status = "unresolved-link";
                        detail = format!(
                            "dependency binding `{}` does not match provider `{}` capability contract",
                            binding.id,
                            provider_name.map(String::as_str).unwrap_or("<missing>")
                        );
                    }
                } else {
                    status = "unresolved-link";
                    detail = format!(
                        "dependency binding `{}` names a provider module that is not in this RMS system",
                        binding.id
                    );
                }
            } else if binding.resolution != "external"
                || binding.provider_module.is_some()
                || binding.provider_contract.is_some()
            {
                status = "unresolved-link";
                detail = format!(
                    "dependency binding `{}` has an invalid resolution declaration",
                    binding.id
                );
            }
        }
        push_obligation(
            graph,
            module,
            "dependency-binding",
            status,
            format!("Resolve required capability `{capability}`"),
            detail,
            vec![source.clone()],
        );
    }
    for binding in bindings {
        if required.contains_key(&binding.capability) {
            continue;
        }
        push_obligation(
            graph,
            module,
            "dependency-binding",
            "unresolved-link",
            format!("Remove stale dependency binding `{}`", binding.id),
            format!(
                "dependency behavior binding `{}` targets undeclared required capability `{}`",
                binding.id, binding.capability
            ),
            vec![source.clone()],
        );
    }
}

fn project_invariant_closure(
    root: &Path,
    module: &ModuleProjection,
    implementation: &LoadedManifest,
    graph: &mut SemanticSystemGraph,
    invariant_ids: &[String],
) {
    let functions = get_path(&implementation.value, &["semantic_functions"])
        .and_then(YamlValue::as_sequence)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let invariant_items = get_path(&module.manifest.value, &["invariants"])
        .and_then(YamlValue::as_sequence)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for invariant_id in invariant_ids {
        let item = invariant_items
            .iter()
            .find(|item| get_str(item, &["id"]) == Some(invariant_id.as_str()));
        let enforced_by = item.and_then(|item| get_str(item, &["enforced_by"]));
        let direct_evidence = item
            .and_then(|item| get_str(item, &["verified_by"]))
            .is_some_and(|path| module.base().join(path).is_file());
        let discharging_functions = functions
            .iter()
            .filter(|function| {
                get_string_array(function, &["discharges", "invariants"]).contains(invariant_id)
                    || enforced_by.is_some_and(|owner| {
                        get_str(function, &["id"]) == Some(owner)
                            || get_str(function, &["symbol"])
                                .and_then(|symbol| symbol.rsplit_once('#').map(|(_, name)| name))
                                == Some(owner)
                            || get_str(function, &["symbol"]) == Some(owner)
                    })
            })
            .collect::<Vec<_>>();
        let function_evidence = discharging_functions.iter().any(|function| {
            get_path(function, &["evidence"])
                .and_then(YamlValue::as_mapping)
                .into_iter()
                .flat_map(|mapping| mapping.values())
                .flat_map(|value| value.as_sequence().into_iter().flatten())
                .filter_map(YamlValue::as_str)
                .any(|path| module.base().join(path).is_file())
        });
        let composition = item.and_then(|item| get_str(item, &["authority"]))
            == Some("composition")
            && get_path(&module.manifest.value, &["verification", "delegations"])
                .and_then(YamlValue::as_sequence)
                .is_some_and(|delegations| {
                    delegations.iter().any(|delegation| {
                        get_str(delegation, &["law"]) == Some(invariant_id.as_str())
                            || get_str(delegation, &["parent_law"]) == Some(invariant_id.as_str())
                    })
                });
        let has_owner = !discharging_functions.is_empty() || composition;
        let has_proof = direct_evidence || function_evidence || composition;
        let fixture_only = get_str(&implementation.value, &["binding"]) == Some("fixture");
        let (status, detail) = if fixture_only && !has_owner {
            (
                "recommendation",
                format!(
                    "fixture invariant `{invariant_id}` has evidence but no inspectable semantic-function owner"
                ),
            )
        } else {
            match (has_owner, has_proof) {
            (true, true) => (
                "satisfied",
                format!("invariant `{invariant_id}` has an authority owner and concrete proof"),
            ),
            (false, _) => (
                "unresolved-link",
                format!("invariant `{invariant_id}` is not discharged by a semantic function or composition delegation"),
            ),
            (_, false) => (
                "required-gap",
                format!("invariant `{invariant_id}` has no concrete evidence in its proof chain"),
            ),
            }
        };
        push_obligation(
            graph,
            module,
            "invariant-proof-chain",
            status,
            format!("Prove invariant `{invariant_id}`"),
            detail,
            vec![module.source(root)],
        );
    }
}

fn project_trace_case_closure(
    root: &Path,
    module: &ModuleProjection,
    graph: &mut SemanticSystemGraph,
    transitions: &[String],
    recorded: &BTreeSet<String>,
) {
    for case in transitions {
        let represented = recorded.contains(case);
        push_obligation(
            graph,
            module,
            "trace-case",
            if represented {
                "satisfied"
            } else {
                "required-gap"
            },
            format!("Replay transition `{case}`"),
            if represented {
                format!("transition case `{case}` appears in an active replay bundle")
            } else {
                format!("transition case `{case}` is not represented in an active replay bundle")
            },
            vec![module.source(root)],
        );
    }
}

fn project_effect_closure(
    root: &Path,
    module: &ModuleProjection,
    graph: &mut SemanticSystemGraph,
    effects: &[String],
    protocols: &[MachineEffectProtocol],
) {
    for effect in effects {
        let owner = protocols.iter().find(|protocol| {
            protocol.effect == *effect
                && protocol
                    .executor_role
                    .as_deref()
                    .is_some_and(|role| !role.is_empty())
                && protocol
                    .executor_symbol
                    .as_deref()
                    .is_some_and(|symbol| !symbol.is_empty())
        });
        push_obligation(
            graph,
            module,
            "effect-owner",
            if owner.is_some() {
                "satisfied"
            } else {
                "required-gap"
            },
            format!("Own effect `{effect}`"),
            if owner.is_some() {
                format!("effect `{effect}` has an exact protocol and executor owner")
            } else {
                format!("effect `{effect}` has no exact effect protocol and executor owner")
            },
            vec![module.source(root)],
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn project_shape_obligations(
    root: &Path,
    module: &ModuleProjection,
    graph: &mut SemanticSystemGraph,
    public_behaviors: &BTreeMap<(String, String), String>,
    required_capabilities: &BTreeMap<String, Option<String>>,
    implemented: bool,
    machine_effects: &[String],
    transitions: &[String],
    trace_cases: &BTreeSet<String>,
) {
    let source = vec![module.source(root)];
    let boundary_applicable = module.shape.as_deref() == Some("boundary-adapter")
        || module.module_kind == "adapter"
        || get_string_array(&module.manifest.value, &["profiles"])
            .iter()
            .any(|profile| profile == "boundary");
    let boundary_satisfied = get_path(&module.manifest.value, &["boundary"]).is_some()
        || module
            .implementation
            .as_ref()
            .is_some_and(|implementation| {
                get_path(&implementation.value, &["architecture", "surfaces"])
                    .and_then(YamlValue::as_sequence)
                    .is_some_and(|items| !items.is_empty())
                    || get_path(&implementation.value, &["architecture", "roles", "parser"])
                        .and_then(YamlValue::as_sequence)
                        .is_some_and(|items| !items.is_empty())
            });
    push_obligation(
        graph,
        module,
        "boundary",
        applicability_status(boundary_applicable, boundary_satisfied),
        "Boundary semantics".to_string(),
        applicability_detail(
            boundary_applicable,
            boundary_satisfied,
            "boundary parsing/delegation is declared",
            "this module shape has no untrusted runnable boundary",
            "this boundary-shaped module has no parser, boundary declaration, or runnable surface",
        ),
        source.clone(),
    );
    let stateful = module
        .implementation
        .as_ref()
        .is_some_and(|implementation| {
            get_str(&implementation.value, &["architecture", "machine", "mode"])
                .is_some_and(|mode| mode != "stateless-decision-machine")
        });
    push_obligation(
        graph,
        module,
        "lifecycle",
        applicability_status(stateful, !transitions.is_empty()),
        "Lifecycle model".to_string(),
        applicability_detail(
            stateful,
            !transitions.is_empty(),
            "stateful lifecycle transitions are declared",
            "stateless or semantic-only behavior does not require lifecycle state",
            "stateful machine has no declared transition cases",
        ),
        source.clone(),
    );
    let effect_applicable = !machine_effects.is_empty()
        || get_path(&module.manifest.value, &["effects"])
            .and_then(YamlValue::as_sequence)
            .is_some_and(|items| !items.is_empty());
    push_obligation(
        graph,
        module,
        "effects",
        if effect_applicable {
            if machine_effects.is_empty() && implemented {
                "required-gap"
            } else {
                "satisfied"
            }
        } else {
            "not-applicable"
        },
        "Effect semantics".to_string(),
        if effect_applicable {
            "declared effects belong to the machine/effect protocol model".to_string()
        } else {
            "pure modules and composites without IO do not require effect declarations".to_string()
        },
        source.clone(),
    );
    let proof_applicable = !public_behaviors.is_empty()
        || get_path(&module.manifest.value, &["invariants"])
            .and_then(YamlValue::as_sequence)
            .is_some_and(|items| !items.is_empty());
    let proof_satisfied = get_path(&module.manifest.value, &["verification"])
        .and_then(YamlValue::as_mapping)
        .is_some_and(|verification| {
            verification.values().any(|value| {
                value
                    .as_sequence()
                    .is_some_and(|references| !references.is_empty())
            })
        });
    push_obligation(
        graph,
        module,
        "proof",
        applicability_status(proof_applicable, proof_satisfied),
        "Proof evidence".to_string(),
        applicability_detail(
            proof_applicable,
            proof_satisfied,
            "declared promises have evidence lanes",
            "no implemented public or invariant-bearing promise requires proof",
            "declared promises have no evidence references",
        ),
        source.clone(),
    );
    push_obligation(
        graph,
        module,
        "public-binding-applicability",
        if !implemented || public_behaviors.is_empty() {
            "not-applicable"
        } else {
            "satisfied"
        },
        "Public behavior binding applicability".to_string(),
        if !implemented {
            "semantic-only and composite modules bind exports through composition, not implementation symbols".to_string()
        } else if public_behaviors.is_empty() {
            "this implementation exposes no public behavior".to_string()
        } else {
            "implemented public behavior requires exact bindings".to_string()
        },
        source.clone(),
    );
    push_obligation(
        graph,
        module,
        "dependency-binding-applicability",
        if !implemented || required_capabilities.is_empty() {
            "not-applicable"
        } else {
            "satisfied"
        },
        "Dependency binding applicability".to_string(),
        if required_capabilities.is_empty() {
            "this module has no required capabilities".to_string()
        } else {
            "implemented required capabilities need exact consumer bindings".to_string()
        },
        source.clone(),
    );
    let trace_applicable = !transitions.is_empty();
    push_obligation(
        graph,
        module,
        "trace",
        applicability_status(
            trace_applicable,
            transitions.iter().all(|case| trace_cases.contains(case)),
        ),
        "Replay coverage".to_string(),
        applicability_detail(
            trace_applicable,
            transitions.iter().all(|case| trace_cases.contains(case)),
            "all declared transition cases appear in replay evidence",
            "no transition cases require replay evidence",
            "one or more declared transition cases are absent from replay evidence",
        ),
        source,
    );
}

fn machine_transition_cases(value: &YamlValue) -> Vec<String> {
    get_path(value, &["architecture", "machine", "transitions"])
        .and_then(YamlValue::as_sequence)
        .into_iter()
        .flatten()
        .filter_map(|item| get_str(item, &["case"]).map(ToString::to_string))
        .collect()
}

fn recorded_trace_cases(base: &Path, value: &YamlValue) -> BTreeSet<String> {
    let mut cases = BTreeSet::new();
    let Some(producers) =
        get_path(value, &["architecture", "trace", "producers"]).and_then(YamlValue::as_sequence)
    else {
        return cases;
    };
    for producer in producers {
        let Some(bundle) = get_str(producer, &["bundle"]) else {
            continue;
        };
        let Ok(contents) = fs::read_to_string(base.join(bundle)) else {
            continue;
        };
        let Ok(document) = serde_yaml::from_str::<YamlValue>(&contents) else {
            continue;
        };
        let artifact_spec = get_str(&document, &["spec"]).unwrap_or("");
        let probe_trace = match artifact_spec {
            "rms/probe-system-trace/v0.1" => Some(&document),
            "rms/probe-counterexample/v0.1" => get_path(&document, &["trace"]),
            _ => None,
        };
        if let Some(trace) = probe_trace {
            if let Some(entries) = get_path(trace, &["timeline"]).and_then(YamlValue::as_sequence) {
                for entry in entries {
                    if let Some(case) = get_str(entry, &["transition_case"]) {
                        cases.insert(case.to_string());
                    }
                }
            }
            continue;
        }
        let Some(records) = get_path(&document, &["records"]).and_then(YamlValue::as_sequence)
        else {
            continue;
        };
        for record in records {
            if let Some(case) = get_str(record, &["source", "branch"])
                .or_else(|| get_str(record, &["source", "case"]))
            {
                cases.insert(case.to_string());
            }
        }
    }
    cases
}

fn input_kind<'a>(implementation: &YamlValue, input: &'a str) -> &'a str {
    let variant = input.rsplit('.').next().unwrap_or(input);
    if get_string_array(
        implementation,
        &["architecture", "machine", "observed_events"],
    )
    .iter()
    .any(|item| item == variant)
    {
        "observed-event"
    } else if get_string_array(
        implementation,
        &["architecture", "machine", "effect_results"],
    )
    .iter()
    .any(|item| item == variant)
    {
        "effect-result"
    } else {
        "command"
    }
}

fn applicability_status(applicable: bool, satisfied: bool) -> &'static str {
    if !applicable {
        "not-applicable"
    } else if satisfied {
        "satisfied"
    } else {
        "required-gap"
    }
}

fn applicability_detail(
    applicable: bool,
    satisfied: bool,
    success: &str,
    not_applicable: &str,
    gap: &str,
) -> String {
    if !applicable {
        not_applicable.to_string()
    } else if satisfied {
        success.to_string()
    } else {
        gap.to_string()
    }
}

fn graph_id(module: &str, kind: &str, value: &str) -> String {
    stable_atlas_id(kind, &format!("{module}:{value}"))
}

fn source_ref(root: &Path, path: &Path, role: &str) -> SemanticGraphSourceRef {
    SemanticGraphSourceRef {
        role: role.to_string(),
        path: display_relative(root, path),
    }
}

fn push_node(graph: &mut SemanticSystemGraph, ids: &mut BTreeSet<String>, node: SemanticGraphNode) {
    if ids.insert(node.id.clone()) {
        graph.nodes.push(node);
    }
}

fn push_edge(
    graph: &mut SemanticSystemGraph,
    ids: &mut BTreeSet<String>,
    kind: &str,
    from: &str,
    to: &str,
    label: &str,
    source_refs: Vec<SemanticGraphSourceRef>,
) {
    let id = stable_atlas_id("edge", &format!("{kind}:{from}:{to}:{label}"));
    if ids.insert(id.clone()) {
        graph.edges.push(SemanticGraphEdge {
            id,
            kind: kind.to_string(),
            from: from.to_string(),
            to: to.to_string(),
            label: label.to_string(),
            source_refs,
        });
    }
}

fn push_obligation(
    graph: &mut SemanticSystemGraph,
    module: &ModuleProjection,
    kind: &str,
    status: &'static str,
    title: String,
    detail: String,
    source_refs: Vec<SemanticGraphSourceRef>,
) {
    let id = stable_atlas_id(
        "obligation",
        &format!("{}:{kind}:{title}", module.module_name),
    );
    graph.obligations.push(SemanticGraphObligation {
        id,
        module_id: module.module_id.clone(),
        kind: kind.to_string(),
        status,
        title,
        detail,
        source_refs,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn applicability_distinguishes_absence_from_a_gap() {
        assert_eq!(applicability_status(false, false), "not-applicable");
        assert_eq!(applicability_status(true, false), "required-gap");
        assert_eq!(applicability_status(true, true), "satisfied");
    }

    #[test]
    fn pure_machine_projects_inner_semantics_without_effect_gap() {
        let root = fixture_root("pure-machine");
        write_fixture(
            &root.join("module.yaml"),
            r#"spec: rms/module/v0.1
module: { name: chooser, version: 0.1.0, kind: library, purpose: Choose safely }
profiles: [core]
owns: { concepts: [], data: [], decisions: [] }
provides: { commands: [], queries: [], events: [], capabilities: [] }
requires: { modules: [], capabilities: [] }
invariants: []
effects: []
compatibility: { policy: backward-compatible-within-major }
verification: { laws: [], contracts: [], scenarios: [], boundaries: [] }
"#,
        );
        write_fixture(
            &root.join("implementation.yaml"),
            r#"spec: rms/implementation/v0.1
module: chooser
binding: fixture
source: { root: ., public_entrypoint: src/lib }
commands: { build: noop, verify: noop }
architecture:
  shape: domain-engine
  machine:
    name: ChooserMachine
    mode: stateless-decision-machine
    states: [Ready]
    commands: [Choose]
    observed_events: []
    events: [Chosen]
    effects: []
    effect_results: []
    replies: [Choice]
    rejections: [InvalidChoice]
    transitions:
      - { from: Ready, on: Choose, to: Ready, case: choose, events: [Chosen], reply: Choice }
semantic_functions: []
"#,
        );
        let graph = build_semantic_system_graph(&root).unwrap();
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == "machine" && node.label == "ChooserMachine"));
        assert!(graph
            .nodes
            .iter()
            .any(|node| node.kind == "state" && node.label == "Ready"));
        assert!(graph
            .obligations
            .iter()
            .any(|item| item.kind == "effects" && item.status == "not-applicable"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn implemented_public_command_requires_an_exact_behavior_binding() {
        let root = fixture_root("public-binding-gap");
        write_public_binding_fixture(&root, false);
        let diagnostics = semantic_system_graph_diagnostics(&root).unwrap();
        assert!(diagnostics
            .iter()
            .any(|item| item.check == "semantic.public-binding-missing"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn canonical_probe_trace_projects_a_plain_causal_timeline() {
        let root = fixture_root("probe-timeline");
        write_fixture(
            &root.join("module.yaml"),
            r#"spec: rms/module/v0.1
module: { name: probe-view, version: 0.1.0, kind: library, purpose: Show probe evidence }
profiles: [core]
owns: { concepts: [], data: [], decisions: [] }
provides: { commands: [], queries: [], events: [], capabilities: [] }
requires: { modules: [], capabilities: [] }
invariants: []
effects: []
compatibility: { policy: backward-compatible-within-major }
verification: { laws: [], contracts: [], scenarios: [], boundaries: [] }
"#,
        );
        write_fixture(
            &root.join("implementation.yaml"),
            r#"spec: rms/implementation/v0.1
module: probe-view
binding: fixture
source: { root: ., public_entrypoint: src/lib }
commands: { probe-regression: noop }
architecture:
  shape: domain-engine
  trace:
    producers:
      - id: checkout-integration
        profile: smoke
        bundle: verification/traces/checkout-probe.json
        command: probe-regression
        runner: probes/checkout.yaml#explore
  machine:
    name: ProbeViewMachine
    mode: stateful-transition-machine
    states: [Ready, Waiting]
    commands: [Start]
    observed_events: []
    events: []
    effects: []
    effect_results: [Done]
    replies: []
    rejections: []
    transitions:
      - { from: Ready, on: Start, to: Waiting, case: StartWork }
semantic_functions: []
"#,
        );
        write_fixture(
            &root.join("verification/traces/checkout-probe.json"),
            r#"{
  "spec": "rms/probe-system-trace/v0.1",
  "result": "pass",
  "mode": "exploration",
  "exhausted": true,
  "assembly_digest": "assembly-1",
  "instances": [],
  "timeline": [{
    "step": 1,
    "time": 0,
    "action": "deliver",
    "envelope": "start",
    "idempotency_key": "start",
    "route": "stimulus/start",
    "source": null,
    "target": "source",
    "input": {"kind":"command","name":"Start","data":{}},
    "correlation_id": "start",
    "causation_id": null,
    "attempt": 1,
    "state_before": {"name":"Ready"},
    "state_after": {"name":"Waiting"},
    "transition_case": "StartWork",
    "outputs": []
  }],
  "protocols": {},
  "checks": [],
  "failure": null,
  "coverage": {"states":2,"schedules":1,"transitions":1,"transition_cases":["source:StartWork"],"routes":["stimulus/start"],"faults":[]},
  "bounds": {"max_steps":30,"max_schedules":100,"max_states":10000}
}"#,
        );

        let graph = build_semantic_system_graph(&root).unwrap();
        let bundle = graph
            .nodes
            .iter()
            .find(|node| node.kind == "trace-bundle")
            .expect("probe trace bundle");
        assert_eq!(
            bundle.details.get("result").map(String::as_str),
            Some("pass")
        );
        let step = graph
            .nodes
            .iter()
            .find(|node| node.kind == "trace-record")
            .expect("probe timeline step");
        assert_eq!(
            step.details.get("correlation").map(String::as_str),
            Some("start")
        );
        assert_eq!(
            step.details.get("idempotency").map(String::as_str),
            Some("start")
        );
        assert!(graph
            .obligations
            .iter()
            .any(|item| item.kind == "trace" && item.status == "satisfied"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn exact_public_binding_closes_contract_to_machine_path() {
        let root = fixture_root("public-binding-closed");
        write_public_binding_fixture(&root, true);
        let diagnostics = semantic_system_graph_diagnostics(&root).unwrap();
        assert!(!diagnostics.iter().any(|item| {
            matches!(
                item.check.as_str(),
                "semantic.public-binding-missing" | "semantic.public-input-unreachable"
            )
        }));
        let graph = build_semantic_system_graph(&root).unwrap();
        assert!(graph.edges.iter().any(|edge| edge.kind == "bound-through"));
        assert!(graph.edges.iter().any(|edge| edge.kind == "maps-to"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn composite_capability_export_resolves_its_public_declaration() {
        let root = fixture_root("composite-capability-export");
        write_fixture(
            &root.join("parent/module.yaml"),
            r#"spec: rms/module/v0.1
module: { name: parent, version: 0.1.0, kind: composite, purpose: Export child behavior }
profiles: [core]
owns: { concepts: [], data: [], decisions: [] }
provides:
  commands: []
  queries: []
  events: []
  capabilities:
    - { name: playout-liveness, contract: contracts/playout-liveness.v1.yaml }
requires: { modules: [], capabilities: [] }
composition:
  contains:
    - { name: child, visibility: internal, path: ../child/module.yaml }
  exports:
    - { group: capabilities, name: playout-liveness, from: child }
invariants: []
effects: []
compatibility: { policy: backward-compatible-within-major }
verification: { laws: [], contracts: [], scenarios: [], boundaries: [] }
"#,
        );
        write_fixture(
            &root.join("child/module.yaml"),
            r#"spec: rms/module/v0.1
module: { name: child, version: 0.1.0, kind: library, purpose: Own child behavior }
profiles: [core]
owns: { concepts: [], data: [], decisions: [] }
provides:
  commands: []
  queries: []
  events: []
  capabilities:
    - { name: playout-liveness, contract: contracts/playout-liveness.v1.yaml }
requires: { modules: [], capabilities: [] }
invariants: []
effects: []
compatibility: { policy: backward-compatible-within-major }
verification: { laws: [], contracts: [], scenarios: [], boundaries: [] }
"#,
        );

        let diagnostics = semantic_system_graph_diagnostics(&root).unwrap();
        assert!(!diagnostics
            .iter()
            .any(|item| item.check == "semantic.module-link-unresolved"));
        let graph = build_semantic_system_graph(&root).unwrap();
        assert!(graph
            .edges
            .iter()
            .any(|edge| edge.kind == "exports" && edge.label == "exports capability"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn public_binding_requires_concrete_evidence_to_close_its_proof_chain() {
        let root = fixture_root("public-proof-chain-gap");
        write_public_binding_fixture(&root, true);
        fs::remove_file(root.join("verification/contracts/choose.md")).unwrap();

        let diagnostics = semantic_system_graph_diagnostics(&root).unwrap();
        assert!(diagnostics
            .iter()
            .any(|item| item.check == "semantic.public-proof-chain-incomplete"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn public_binding_requires_its_semantic_function_to_discharge_the_contract() {
        let root = fixture_root("public-binding-undischarged");
        write_public_binding_fixture(&root, true);
        let path = root.join("implementation.yaml");
        let implementation = fs::read_to_string(&path)
            .unwrap()
            .replace("contracts: [contracts/choose.v1.yaml]", "contracts: []");
        fs::write(path, implementation).unwrap();

        let diagnostics = semantic_system_graph_diagnostics(&root).unwrap();
        assert!(diagnostics
            .iter()
            .any(|item| item.check == "semantic.public-binding-invalid"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn stale_public_binding_is_an_unresolved_link() {
        let root = fixture_root("stale-public-binding");
        write_public_binding_fixture(&root, true);
        let path = root.join("implementation.yaml");
        let implementation = fs::read_to_string(&path)
            .unwrap()
            .replace("public_name: choose", "public_name: removed-command");
        fs::write(path, implementation).unwrap();

        let diagnostics = semantic_system_graph_diagnostics(&root).unwrap();
        assert!(diagnostics
            .iter()
            .any(|item| item.check == "semantic.public-binding-invalid"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn dependency_binding_requires_an_exact_consumer_symbol() {
        let root = fixture_root("dependency-consumer-gap");
        write_fixture(
            &root.join("module.yaml"),
            r#"spec: rms/module/v0.1
module: { name: consumer, version: 0.1.0, kind: adapter, purpose: Consume a capability }
profiles: [boundary]
owns: { concepts: [], data: [], decisions: [] }
provides: { commands: [], queries: [], events: [], capabilities: [] }
requires:
  modules: []
  capabilities:
    - { name: filesystem }
invariants: []
effects: []
compatibility: { policy: backward-compatible-within-major }
verification: { laws: [], contracts: [], scenarios: [], boundaries: [] }
"#,
        );
        write_fixture(
            &root.join("implementation.yaml"),
            r#"spec: rms/implementation/v0.1
module: consumer
binding: fixture
source: { root: ., public_entrypoint: src/lib }
commands: { build: noop, verify: noop }
architecture:
  shape: boundary-adapter
  dependency_behavior_bindings:
    - id: filesystem-provider
      capability: filesystem
      consumer: src/missing#read_file
      resolution: external
semantic_functions: []
"#,
        );

        let diagnostics = semantic_system_graph_diagnostics(&root).unwrap();
        assert!(diagnostics
            .iter()
            .any(|item| item.check == "semantic.required-capability-binding-missing"));
        let _ = fs::remove_dir_all(root);
    }

    fn write_public_binding_fixture(root: &Path, include_binding: bool) {
        fs::create_dir_all(root.join("contracts")).unwrap();
        fs::create_dir_all(root.join("verification/contracts")).unwrap();
        write_fixture(
            &root.join("module.yaml"),
            r#"spec: rms/module/v0.1
module: { name: chooser, version: 0.1.0, kind: library, purpose: Choose safely }
profiles: [core]
owns: { concepts: [], data: [], decisions: [] }
provides:
  commands:
    - { name: choose, contract: contracts/choose.v1.yaml }
  queries: []
  events: []
  capabilities: []
requires: { modules: [], capabilities: [] }
invariants: []
effects: []
compatibility: { policy: backward-compatible-within-major }
verification: { laws: [], contracts: [], scenarios: [], boundaries: [] }
"#,
        );
        write_fixture(
            &root.join("contracts/choose.v1.yaml"),
            "spec: rms/contract/v0.1\nname: choose\nversion: 1\nkind: command\nmeaning: Choose one value.\n",
        );
        write_fixture(
            &root.join("verification/contracts/choose.md"),
            "# Choose contract\n\nCommand/tool: fixture verify\n\nSource revision: fixture\n",
        );
        let binding = if include_binding {
            r#"
  public_behavior_bindings:
    - id: choose-public
      public_kind: command
      public_name: choose
      contract: contracts/choose.v1.yaml
      semantic_function: choose-transition
      machine_inputs: [Choose]
      machine_outputs: [Choice]
"#
        } else {
            ""
        };
        write_fixture(
            &root.join("implementation.yaml"),
            &format!(
                r#"spec: rms/implementation/v0.1
module: chooser
binding: fixture
source: {{ root: ., public_entrypoint: src/lib }}
commands: {{ build: noop, verify: noop }}
architecture:
  shape: domain-engine
  machine:
    name: ChooserMachine
    mode: stateless-decision-machine
    states: [Ready]
    commands: [Choose]
    observed_events: []
    events: []
    effects: []
    effect_results: []
    replies: [Choice]
    rejections: []
    transitions: []
{binding}
semantic_functions:
  - id: choose-transition
    symbol: choose
    kind: transition
    purity: pure
    discharges:
      contracts: [contracts/choose.v1.yaml]
    evidence:
      contracts: [verification/contracts/choose.md]
"#
            ),
        );
    }

    fn fixture_root(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("rms-semantic-graph-{label}-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    fn write_fixture(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}

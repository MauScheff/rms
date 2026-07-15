use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_yaml::Value as YamlValue;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use super::semantic_graph::{build_semantic_system_graph, SemanticSystemGraph};
use super::viewer_request::{parse_view_request, ViewRoute};
use super::{
    build_module_atlas, discover_module_manifests, get_str, source_revision, stable_atlas_id,
    validate_loaded_manifest, AtlasDocument, Diagnostic, LoadedManifest, Severity,
    VALIDATOR_VERSION,
};

const VIEWER_HTML: &str = include_str!("viewer_template.html");
const VIEWER_APP: &str = include_str!("viewer_app.js");
const MAX_REQUEST_BYTES: usize = 16 * 1024;

#[derive(Debug, Serialize)]
pub(super) struct SystemViewDocument {
    spec: &'static str,
    source: SystemViewSource,
    system: SystemViewSummary,
    graph: SemanticSystemGraph,
    journeys: Vec<SystemViewJourney>,
    modules: Vec<SystemViewModule>,
    relationships: Vec<SystemViewRelationship>,
    gaps: Vec<SystemViewGap>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
struct SystemViewSource {
    root: String,
    source_revision: Option<String>,
    generated_by: &'static str,
    generated_version: &'static str,
    refresh_ms: u64,
    authority: &'static str,
}

#[derive(Debug, Serialize)]
struct SystemViewSummary {
    name: String,
    purpose: Option<String>,
    module_count: usize,
    relationship_count: usize,
    semantic_node_count: usize,
    trace_count: usize,
    gap_count: usize,
    diagnostic_count: usize,
}

#[derive(Debug, Serialize)]
struct SystemViewJourney {
    id: &'static str,
    label: &'static str,
    focus: &'static str,
}

#[derive(Debug, Serialize)]
struct SystemViewModule {
    id: String,
    name: String,
    kind: String,
    purpose: String,
    manifest_path: String,
    atlas: AtlasDocument,
}

#[derive(Debug, Serialize)]
struct SystemViewRelationship {
    id: String,
    kind: &'static str,
    from: String,
    to: String,
    label: String,
    source: SystemViewSourceRef,
}

#[derive(Debug, Serialize)]
struct SystemViewGap {
    id: String,
    module_id: String,
    kind: &'static str,
    title: String,
    detail: String,
    source: SystemViewSourceRef,
}

#[derive(Debug, Serialize)]
struct SystemViewSourceRef {
    role: &'static str,
    path: String,
}

pub fn run_view(root: &Path, port: u16, watch: bool, open: bool) -> Result<()> {
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve RMS viewer root `{}`", root.display()))?;
    let initial = build_system_view(&root, watch)?;
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
        .with_context(|| format!("failed to bind RMS viewer to 127.0.0.1:{port}"))?;
    let address = listener.local_addr()?;
    let url = format!("http://{address}/");

    println!("RMS Explorer: {url}");
    println!("root: {}", root.display());
    println!("modules: {}", initial.system.module_count);
    println!(
        "mode: read-only{}",
        if watch { " + live refresh" } else { "" }
    );
    println!("press Ctrl-C to stop");

    if open {
        if let Err(error) = open_browser(&url) {
            eprintln!("warning: could not open browser automatically: {error:#}");
        }
    }

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(error) = handle_connection(stream, &root, watch) {
                    eprintln!("viewer request failed: {error:#}");
                }
            }
            Err(error) => eprintln!("viewer connection failed: {error}"),
        }
    }
    Ok(())
}

pub(super) fn build_system_view(root: &Path, watch: bool) -> Result<SystemViewDocument> {
    let mut manifests = discover_module_manifests(root)?;
    if manifests.is_empty() {
        bail!(
            "RMS viewer found no canonical module manifests under `{}`",
            root.display()
        );
    }
    manifests.sort_by(|left, right| module_name(left).cmp(&module_name(right)));

    let mut diagnostics = Vec::new();
    for manifest in &manifests {
        validate_loaded_manifest(manifest, &mut diagnostics);
    }
    let error_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    if error_count > 0 {
        bail!("RMS viewer rejected {error_count} canonical validation error(s)");
    }

    let graph = build_semantic_system_graph(root)?;
    let relationships = build_system_relationships(&graph);
    let mut gaps = Vec::new();
    for obligation in &graph.obligations {
        if !matches!(obligation.status, "required-gap" | "unresolved-link") {
            continue;
        }
        let source = obligation.source_refs.first();
        gaps.push(SystemViewGap {
            id: obligation.id.clone(),
            module_id: obligation.module_id.clone(),
            kind: "semantic-obligation",
            title: obligation.title.clone(),
            detail: obligation.detail.clone(),
            source: SystemViewSourceRef {
                role: "semantic-system-graph",
                path: source
                    .map(|source| source.path.clone())
                    .unwrap_or_else(|| ".".to_string()),
            },
        });
    }
    let mut modules = Vec::new();
    let semantic_node_count = graph.nodes.len();
    let mut trace_count = 0usize;

    for manifest in &manifests {
        let atlas = build_module_atlas(manifest, root)?;
        trace_count = trace_count.saturating_add(atlas.traces.len());
        modules.push(SystemViewModule {
            id: atlas.module.id.clone(),
            name: atlas.module.name.clone(),
            kind: atlas.module.kind.clone(),
            purpose: atlas.module.purpose.clone(),
            manifest_path: relative_path(root, &manifest.path),
            atlas,
        });
    }
    gaps.sort_by(|left, right| left.id.cmp(&right.id));
    gaps.dedup_by(|left, right| left.id == right.id);

    let (name, purpose) = system_identity(root, modules.len());
    let system = SystemViewSummary {
        name,
        purpose,
        module_count: modules.len(),
        relationship_count: relationships.len(),
        semantic_node_count,
        trace_count,
        gap_count: gaps.len(),
        diagnostic_count: diagnostics.len(),
    };

    Ok(SystemViewDocument {
        spec: "rms/view/v0.1",
        source: SystemViewSource {
            root: root.display().to_string(),
            source_revision: source_revision(root),
            generated_by: "rms view",
            generated_version: VALIDATOR_VERSION,
            refresh_ms: if watch { 1_500 } else { 0 },
            authority: "read-only projection of canonical RMS artifacts",
        },
        system,
        graph,
        journeys: view_journeys(),
        modules,
        relationships,
        gaps,
        diagnostics,
    })
}

fn handle_connection(mut stream: TcpStream, root: &Path, watch: bool) -> Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let mut buffer = vec![0u8; MAX_REQUEST_BYTES];
    let bytes_read = stream.read(&mut buffer)?;
    if bytes_read == 0 {
        return Ok(());
    }
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let Some(request_line) = request.lines().next() else {
        return write_response(
            &mut stream,
            400,
            "text/plain; charset=utf-8",
            b"bad request",
            false,
        );
    };
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");

    match parse_view_request(method, target) {
        Ok(parsed) => {
            let (content_type, body) = match parsed.route() {
                ViewRoute::Index => ("text/html; charset=utf-8", VIEWER_HTML.as_bytes().to_vec()),
                ViewRoute::App => (
                    "text/javascript; charset=utf-8",
                    VIEWER_APP.as_bytes().to_vec(),
                ),
                ViewRoute::Snapshot => (
                    "application/json; charset=utf-8",
                    serde_json::to_vec(&build_system_view(root, watch)?)?,
                ),
                ViewRoute::Health => (
                    "application/json; charset=utf-8",
                    br#"{"status":"ok","authority":"read-only"}"#.to_vec(),
                ),
            };
            write_response(&mut stream, 200, content_type, &body, parsed.head_only())
        }
        Err(rejection) => write_response(
            &mut stream,
            rejection.status(),
            "application/json; charset=utf-8",
            format!(r#"{{"error":"{}"}}"#, rejection.reason()).as_bytes(),
            method == "HEAD",
        ),
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'self' data:\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()?;
    Ok(())
}

fn build_system_relationships(graph: &SemanticSystemGraph) -> Vec<SystemViewRelationship> {
    let nodes = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let mut relationships = Vec::new();
    let mut seen = BTreeSet::new();
    for edge in &graph.edges {
        let Some(from_node) = nodes.get(edge.from.as_str()) else {
            continue;
        };
        let Some(to_node) = nodes.get(edge.to.as_str()) else {
            continue;
        };
        let relationship = match edge.kind.as_str() {
            "contains" | "requires-module"
                if from_node.kind == "module" && to_node.kind == "module" =>
            {
                Some((edge.kind.as_str(), edge.from.as_str(), edge.to.as_str()))
            }
            "exports" if from_node.kind == "module" => {
                Some(("exports", edge.from.as_str(), to_node.module_id.as_str()))
            }
            "delegates-to"
                if from_node.kind == "dependency-behavior-binding" && to_node.kind == "module" =>
            {
                Some((
                    "requires-capability",
                    from_node.module_id.as_str(),
                    edge.to.as_str(),
                ))
            }
            _ => None,
        };
        let Some((kind, from, to)) = relationship else {
            continue;
        };
        let id = stable_atlas_id("system-edge", &format!("{kind}:{from}:{to}:{}", edge.label));
        if !seen.insert(id.clone()) {
            continue;
        }
        let source = edge.source_refs.first();
        relationships.push(SystemViewRelationship {
            id,
            kind: match kind {
                "contains" => "contains",
                "requires-module" => "requires-module",
                "exports" => "exports",
                _ => "requires-capability",
            },
            from: from.to_string(),
            to: to.to_string(),
            label: edge.label.clone(),
            source: SystemViewSourceRef {
                role: "semantic-system-graph",
                path: source
                    .map(|source| source.path.clone())
                    .unwrap_or_else(|| ".".to_string()),
            },
        });
    }
    relationships.sort_by(|left, right| left.id.cmp(&right.id));
    relationships
}

fn module_name(manifest: &LoadedManifest) -> String {
    get_str(&manifest.value, &["module", "name"])
        .unwrap_or("unnamed-module")
        .to_string()
}

fn system_identity(root: &Path, module_count: usize) -> (String, Option<String>) {
    let system_path = root.join("system.yaml");
    if let Ok(source) = fs::read_to_string(&system_path) {
        if let Ok(value) = serde_yaml::from_str::<YamlValue>(&source) {
            let name = get_str(&value, &["system", "name"])
                .unwrap_or("RMS system")
                .to_string();
            let purpose = get_str(&value, &["system", "purpose"]).map(ToOwned::to_owned);
            return (name, purpose);
        }
    }
    let name = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("RMS system")
        .to_string();
    (
        name,
        Some(format!(
            "Semantic projection of {module_count} canonical RMS module(s)."
        )),
    )
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn view_journeys() -> Vec<SystemViewJourney> {
    vec![
        SystemViewJourney {
            id: "understand",
            label: "Understand",
            focus: "Purpose, ownership, public surfaces, and module composition.",
        },
        SystemViewJourney {
            id: "trace",
            label: "Trace",
            focus: "Commands, transitions, effects, outcomes, and source-backed evidence.",
        },
        SystemViewJourney {
            id: "change",
            label: "Change",
            focus: "Contracts, dependencies, invariants, compatibility, and affected proof.",
        },
        SystemViewJourney {
            id: "debug",
            label: "Debug",
            focus: "Explicit gaps, rejected paths, trace confidence, and first bad transitions.",
        },
        SystemViewJourney {
            id: "verify",
            label: "Verify",
            focus: "Laws, properties, traces, packages, diagnostics, and production readiness.",
        },
    ]
}

fn open_browser(url: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = Command::new("open");
    #[cfg(target_os = "linux")]
    let mut command = Command::new("xdg-open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", ""]);
        command
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    bail!("automatic browser opening is unsupported on this platform");

    let status = command.arg(url).status()?;
    if !status.success() {
        bail!("browser opener exited with status {status}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_view_projects_discovered_modules_with_stable_relationships() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let root = manifest_dir
            .parent()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .expect("repository root");
        let view = build_system_view(root, false).unwrap();
        assert_eq!(view.spec, "rms/view/v0.1");
        assert!(view.modules.iter().any(|module| module.name == "rms-cli"));
        assert!(view.system.semantic_node_count > 0);
        assert_eq!(view.graph.spec, "rms/semantic-system-graph/v0.1");
        assert!(view.graph.nodes.iter().any(|node| node.kind == "machine"));
        assert!(view.graph.nodes.iter().any(|node| {
            node.kind == "public-behavior-binding" && node.module_id.contains("rms-cli")
        }));
        assert!(view.relationships.iter().any(|relationship| {
            relationship.kind == "requires-capability"
                && relationship.from.contains("tic-tac-toe-cli")
                && relationship.to.contains("tic-tac-toe-rules")
        }));
        assert!(!view.gaps.iter().any(|gap| gap.title.contains("apply-move")));
        assert!(view.graph.obligations.iter().any(|obligation| {
            obligation.kind == "effects" && obligation.status == "not-applicable"
        }));
        assert!(view
            .graph
            .obligations
            .iter()
            .filter(|obligation| obligation.status == "not-applicable")
            .all(|obligation| !view.gaps.iter().any(|gap| gap.id == obligation.id)));
        assert_eq!(view.source.refresh_ms, 0);
    }

    #[test]
    fn viewer_model_preserves_exact_graph_paths_statuses_diffs_and_deep_links() {
        let app = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/viewer_app.js");
        let script = r#"
const assert = require('node:assert/strict');
const model = require(process.argv[1]);
const snapshot = {
  graph: {
    nodes: [
      { id: 'module:orders', module_id: 'module:orders', kind: 'module', label: 'orders', summary: 'Own orders', source_refs: [] },
      { id: 'public:create-order', module_id: 'module:orders', kind: 'public-command', label: 'create-order', summary: 'Create one order', source_refs: [] },
      { id: 'binding:create-order', module_id: 'module:orders', kind: 'public-behavior-binding', label: 'create-order binding', summary: 'Exact binding', source_refs: [] },
      { id: 'function:create-order', module_id: 'module:orders', kind: 'semantic-function', label: 'createOrder', summary: 'Pure owner', source_refs: [] },
      { id: 'command:create-order', module_id: 'module:orders', kind: 'command', label: 'CreateOrder', summary: 'Machine input', source_refs: [] },
    ],
    edges: [
      { id: 'edge:1', from: 'public:create-order', to: 'binding:create-order', kind: 'bound-by', label: 'bound by', source_refs: [] },
      { id: 'edge:2', from: 'binding:create-order', to: 'function:create-order', kind: 'implemented-by', label: 'implemented by', source_refs: [] },
      { id: 'edge:3', from: 'binding:create-order', to: 'command:create-order', kind: 'accepts-input', label: 'accepts input', source_refs: [] },
    ],
    obligations: [
      { id: 'obligation:gap', module_id: 'module:orders', kind: 'proof', status: 'required-gap', title: 'Proof missing', detail: 'Needs executable proof', source_refs: [] },
      { id: 'obligation:na', module_id: 'module:orders', kind: 'effects', status: 'not-applicable', title: 'Effects', detail: 'Pure module', source_refs: [] },
    ],
  },
};
const index = model.buildIndex(snapshot);
assert.deepEqual(model.neighborhood(index, 'public:create-order').map((item) => item.node.id), [
  'public:create-order', 'binding:create-order', 'function:create-order', 'command:create-order'
]);
assert.equal(model.moduleStatus(index, 'module:orders'), 'required-gap');
assert.equal(model.obligations(index, { status: 'not-applicable' })[0].id, 'obligation:na');
assert.ok(model.nodes(index, { query: 'CreateOrder' }).some((node) => node.id === 'command:create-order'));
const location = { pathname: '/', search: '?view=machines&node=command%3Acreate-order&module=module%3Aorders&status=required-gap&q=order' };
const state = model.parseUrl(location);
assert.equal(state.view, 'machines');
assert.equal(state.nodeId, 'command:create-order');
assert.equal(model.parseUrl({ search: model.urlFor(state, location).slice(1) }).status, 'required-gap');
const changed = structuredClone(snapshot);
changed.graph.nodes[4].summary = 'Changed machine input';
changed.graph.nodes.push({ id: 'state:ready', module_id: 'module:orders', kind: 'state', label: 'Ready', summary: 'Ready', source_refs: [] });
changed.graph.obligations = changed.graph.obligations.slice(1);
assert.deepEqual(model.semanticDiff(snapshot, changed), { added: 1, changed: 1, removed: 1, unresolved: 0 });
"#;
        let output = Command::new("node")
            .args(["-e", script])
            .arg(&app)
            .output()
            .expect("node must execute the viewer model test");
        assert!(
            output.status.success(),
            "viewer model test failed:\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

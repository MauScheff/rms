"use strict";

const state = {
  snapshot: null,
  mode: "understand",
  selectedModule: null,
  selectedNode: null,
  selectedTrace: null,
  query: "",
  loading: false,
  pollTimer: null,
  diff: { added: 0, changed: 0, removed: 0 },
};

const elements = {
  systemName: document.querySelector("#system-name"),
  systemPurpose: document.querySelector("#system-purpose"),
  search: document.querySelector("#search"),
  refresh: document.querySelector("#refresh"),
  liveStatus: document.querySelector("#live-status"),
  liveLabel: document.querySelector("#live-label"),
  modes: document.querySelector("#modes"),
  modeContext: document.querySelector("#mode-context"),
  moduleCount: document.querySelector("#module-count"),
  moduleList: document.querySelector("#module-list"),
  stage: document.querySelector("#stage"),
  inspector: document.querySelector("#inspector"),
  inspectorKind: document.querySelector("#inspector-kind"),
  statusMessage: document.querySelector("#status-message"),
  diff: document.querySelector("#diff"),
  revision: document.querySelector("#revision"),
  toast: document.querySelector("#toast"),
};

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function lower(value) {
  return String(value ?? "").toLocaleLowerCase();
}

function currentModule() {
  return state.snapshot?.modules.find((module) => module.id === state.selectedModule) ?? null;
}

function moduleForId(moduleId) {
  return state.snapshot?.modules.find((module) => module.id === moduleId) ?? null;
}

function selectedNodeEntry() {
  if (!state.selectedNode) return null;
  const module = moduleForId(state.selectedNode.moduleId);
  const node = module?.atlas.nodes.find((candidate) => candidate.id === state.selectedNode.nodeId);
  return module && node ? { module, node } : null;
}

function nodeMatches(node, query) {
  if (!query) return true;
  const clauses = (node.clauses ?? []).map((clause) => `${clause.label} ${clause.statement}`).join(" ");
  return lower(`${node.label} ${node.kind} ${node.layer} ${node.summary} ${clauses}`).includes(query);
}

function moduleMatches(module, query) {
  if (!query) return true;
  if (lower(`${module.name} ${module.kind} ${module.purpose}`).includes(query)) return true;
  return module.atlas.nodes.some((node) => nodeMatches(node, query));
}

function semanticIndex(snapshot) {
  const index = new Map();
  for (const module of snapshot.modules) {
    index.set(module.id, JSON.stringify({
      name: module.name,
      kind: module.kind,
      purpose: module.purpose,
      manifest_path: module.manifest_path,
    }));
    for (const node of module.atlas.nodes) {
      index.set(`${module.id}/${node.id}`, JSON.stringify(node));
    }
    for (const edge of module.atlas.edges) {
      index.set(`${module.id}/edge/${edge.id}`, JSON.stringify(edge));
    }
  }
  for (const edge of snapshot.relationships) {
    index.set(`system/${edge.id}`, JSON.stringify(edge));
  }
  return index;
}

function computeDiff(previous, next) {
  if (!previous) return { added: 0, changed: 0, removed: 0 };
  const before = semanticIndex(previous);
  const after = semanticIndex(next);
  let added = 0;
  let changed = 0;
  let removed = 0;
  for (const [id, fingerprint] of after) {
    if (!before.has(id)) added += 1;
    else if (before.get(id) !== fingerprint) changed += 1;
  }
  for (const id of before.keys()) {
    if (!after.has(id)) removed += 1;
  }
  return { added, changed, removed };
}

async function loadSnapshot(manual = false) {
  if (state.loading) return;
  state.loading = true;
  if (manual) elements.statusMessage.textContent = "Refreshing canonical artifacts...";
  try {
    const response = await fetch("/api/snapshot", { cache: "no-store" });
    if (!response.ok) throw new Error(`snapshot request failed (${response.status})`);
    const next = await response.json();
    state.diff = computeDiff(state.snapshot, next);
    state.snapshot = next;
    normalizeSelection();
    render();
    configurePolling();
  } catch (error) {
    elements.stage.innerHTML = `<div class="empty">${escapeHtml(error.message)}</div>`;
    elements.statusMessage.textContent = "Canonical projection unavailable";
    elements.liveStatus.classList.remove("active");
    elements.liveLabel.textContent = "Offline";
  } finally {
    state.loading = false;
  }
}

function configurePolling() {
  if (state.pollTimer) return;
  const refreshMs = state.snapshot?.source.refresh_ms ?? 0;
  if (refreshMs > 0) {
    state.pollTimer = window.setInterval(() => loadSnapshot(false), refreshMs);
  }
}

function normalizeSelection() {
  const modules = state.snapshot.modules;
  if (!modules.some((module) => module.id === state.selectedModule)) {
    state.selectedModule = modules.find((module) => module.kind === "composite")?.id ?? modules[0]?.id ?? null;
    state.selectedNode = null;
    state.selectedTrace = null;
  }
  if (state.selectedNode && !selectedNodeEntry()) state.selectedNode = null;
  const module = currentModule();
  if (state.selectedTrace && !module?.atlas.traces.some((trace) => trace.id === state.selectedTrace)) {
    state.selectedTrace = null;
  }
}

function render() {
  if (!state.snapshot) return;
  renderHeader();
  renderModes();
  renderModules();
  renderStage();
  renderInspector();
  renderStatus();
}

function renderHeader() {
  const { system, source } = state.snapshot;
  elements.systemName.textContent = system.name;
  elements.systemPurpose.textContent = system.purpose ?? "";
  elements.liveStatus.classList.toggle("active", source.refresh_ms > 0);
  elements.liveLabel.textContent = source.refresh_ms > 0 ? "Live" : "Snapshot";
}

function renderModes() {
  elements.modes.innerHTML = state.snapshot.journeys.map((journey) => `
    <button class="mode-button ${journey.id === state.mode ? "active" : ""}" type="button" data-mode="${escapeHtml(journey.id)}">
      ${escapeHtml(journey.label)}
    </button>
  `).join("");
  const journey = state.snapshot.journeys.find((candidate) => candidate.id === state.mode);
  elements.modeContext.textContent = journey?.focus ?? "";
}

function renderModules() {
  const query = lower(state.query.trim());
  const modules = state.snapshot.modules.filter((module) => moduleMatches(module, query));
  elements.moduleCount.textContent = `${modules.length}/${state.snapshot.modules.length}`;
  elements.moduleList.innerHTML = modules.length ? modules.map((module) => `
    <button class="module-button ${module.id === state.selectedModule ? "active" : ""}" type="button" data-module="${escapeHtml(module.id)}">
      <span class="kind-rail ${escapeHtml(module.kind)}"></span>
      <span>
        <span class="module-name">${escapeHtml(module.name)}</span>
        <span class="module-meta">${escapeHtml(module.kind)} &middot; ${module.atlas.nodes.length} semantics</span>
      </span>
    </button>
  `).join("") : `<div class="empty">No modules match the current search.</div>`;
}

function renderStage() {
  const renderers = {
    understand: renderUnderstand,
    trace: renderTrace,
    change: renderChange,
    debug: renderDebug,
    verify: renderVerify,
  };
  elements.stage.innerHTML = (renderers[state.mode] ?? renderUnderstand)();
}

function heading(title, detail, source) {
  return `
    <div class="view-heading">
      <div><h2>${escapeHtml(title)}</h2><p>${escapeHtml(detail)}</p></div>
      ${source ? `<span class="source-chip">${escapeHtml(source)}</span>` : ""}
    </div>
  `;
}

function renderUnderstand() {
  const { system, modules, relationships } = state.snapshot;
  const query = lower(state.query.trim());
  const visible = modules.filter((module) => moduleMatches(module, query));
  return `
    ${heading(system.name, system.purpose ?? "", `${system.module_count} modules`)}
    <div class="stats">
      ${stat(system.module_count, "Modules")}
      ${stat(system.relationship_count, "Relationships")}
      ${stat(system.semantic_node_count, "Semantic objects")}
      ${stat(system.trace_count, "Guided traces")}
      ${stat(system.gap_count, "Explicit gaps")}
    </div>
    <div class="section-label">System topology</div>
    <div class="topology">
      ${visible.map(renderTopologyNode).join("")}
    </div>
    <div class="section-label">Declared relationships</div>
    ${relationships.length ? `
      <div class="relationship-list">
        ${relationships.map((edge) => {
          const from = moduleForId(edge.from)?.name ?? edge.from;
          const to = moduleForId(edge.to)?.name ?? edge.to;
          return `<div class="relationship"><strong>${escapeHtml(from)}</strong><span class="arrow">&rarr;</span><strong>${escapeHtml(to)}</strong><span>${escapeHtml(edge.kind)}: ${escapeHtml(edge.label)}</span></div>`;
        }).join("")}
      </div>
    ` : `<div class="empty">No cross-module relationships are declared in this root.</div>`}
  `;
}

function stat(value, label) {
  return `<div class="stat"><strong>${escapeHtml(value)}</strong><span>${escapeHtml(label)}</span></div>`;
}

function renderTopologyNode(module) {
  const publicNodes = module.atlas.nodes.filter((node) => node.layer === "public-surface").length;
  const proofNodes = module.atlas.nodes.filter((node) => node.layer === "verification").length;
  return `
    <button class="topology-node ${escapeHtml(module.kind)} ${module.id === state.selectedModule ? "active" : ""}" type="button" data-module="${escapeHtml(module.id)}">
      <span class="badge">${escapeHtml(module.kind)}</span>
      <h3>${escapeHtml(module.name)}</h3>
      <p>${escapeHtml(module.purpose)}</p>
      <span class="topology-counts"><span>${publicNodes} public</span><span>${proofNodes} proof</span><span>${module.atlas.traces.length} traces</span></span>
    </button>
  `;
}

function renderTrace() {
  const module = currentModule();
  if (!module) return `<div class="empty">Select a module to inspect its semantic traces.</div>`;
  const traces = module.atlas.traces;
  if (!traces.length) {
    return `${heading(`${module.name} traces`, "No guided semantic traces are declared for this module.", module.manifest_path)}<div class="empty">Trace projection unavailable.</div>`;
  }
  const trace = traces.find((candidate) => candidate.id === state.selectedTrace) ?? traces[0];
  state.selectedTrace = trace.id;
  return `
    ${heading(trace.label, trace.summary, module.name)}
    <div class="trace-layout">
      <div class="trace-list">
        ${traces.map((candidate) => `
          <button class="trace-button ${candidate.id === trace.id ? "active" : ""}" type="button" data-trace="${escapeHtml(candidate.id)}">
            <strong>${escapeHtml(candidate.label)}</strong>
            <span>${candidate.steps.length} steps &middot; ${candidate.gaps.length} gaps</span>
          </button>
        `).join("")}
      </div>
      <div class="trace-steps">
        ${trace.steps.map((step) => renderTraceStep(module, step)).join("")}
        ${trace.gaps.map((gap) => `
          <div class="trace-step">
            <h3>${escapeHtml(gap.title)} <span class="badge gap">gap</span></h3>
            <p>${escapeHtml(gap.body)}</p>
          </div>
        `).join("")}
      </div>
    </div>
  `;
}

function renderTraceStep(module, step) {
  const reading = step.reading ?? {};
  return `
    <div class="trace-step">
      <h3>${escapeHtml(step.title)} <span class="badge ${escapeHtml(step.confidence)}">${escapeHtml(step.confidence)}</span></h3>
      <p>${escapeHtml(step.body)}</p>
      <dl class="reading-grid">
        ${readingItem("Promise", reading.promise)}
        ${readingItem("Before", reading.before)}
        ${readingItem("After", reading.after)}
        ${readingItem("Failure", reading.failure)}
        ${readingItem("Evidence", reading.evidence)}
        ${readingItem("Impact", reading.impact)}
      </dl>
      <div class="inspector-meta">
        ${(step.node_ids ?? []).map((nodeId) => `<button class="badge" type="button" data-module="${escapeHtml(module.id)}" data-node="${escapeHtml(nodeId)}">${escapeHtml(nodeLabel(module, nodeId))}</button>`).join("")}
      </div>
    </div>
  `;
}

function readingItem(label, value) {
  if (!value) return "";
  return `<div class="reading"><dt>${escapeHtml(label)}</dt><dd>${escapeHtml(value)}</dd></div>`;
}

function renderChange() {
  const module = currentModule();
  if (!module) return `<div class="empty">Select a module to inspect change impact.</div>`;
  const query = lower(state.query.trim());
  const priorityLayers = new Set(["public-surface", "constraints", "effects", "lifecycle", "dependencies"]);
  const candidates = module.atlas.nodes
    .filter((node) => priorityLayers.has(node.layer) && nodeMatches(node, query))
    .sort((left, right) => right.emphasis - left.emphasis || left.label.localeCompare(right.label));
  return `
    ${heading(`${module.name} change surface`, "Select a semantic object to see its contract clauses, neighboring dependencies, source ownership, and proof obligations.", module.manifest_path)}
    <div class="semantic-list">
      ${candidates.length ? candidates.map((node) => semanticRow(module, node)).join("") : `<div class="empty">No change surfaces match the current search.</div>`}
    </div>
  `;
}

function renderDebug() {
  const module = currentModule();
  const systemGaps = state.snapshot.gaps.filter((gap) => !module || gap.module_id === module.id);
  const diagnostics = state.snapshot.diagnostics.filter((diagnostic) => !module || diagnostic.path.includes(module.manifest_path));
  return `
    ${heading(module ? `${module.name} diagnostic surface` : "System diagnostic surface", "Gaps are missing semantic links. Diagnostics are evidence about canonical artifacts, never replacement authority.", `${systemGaps.length} gaps`)}
    <div class="section-label">Explicit gaps</div>
    <div class="semantic-list">
      ${systemGaps.length ? systemGaps.map((gap) => `
        <div class="semantic-row">
          <span class="semantic-title">${escapeHtml(gap.title)}</span>
          <span class="semantic-summary">${escapeHtml(gap.detail)}</span>
          <span class="badge gap">${escapeHtml(gap.kind)}</span>
        </div>
      `).join("") : `<div class="empty">No explicit semantic gaps for this scope.</div>`}
    </div>
    <div class="section-label">Canonical diagnostics</div>
    <div class="semantic-list">
      ${diagnostics.length ? diagnostics.map((diagnostic) => `
        <div class="semantic-row">
          <span class="semantic-title">${escapeHtml(diagnostic.check)}</span>
          <span class="semantic-summary">${escapeHtml(diagnostic.message)}</span>
          <span class="badge ${escapeHtml(diagnostic.severity)}">${escapeHtml(diagnostic.severity)}</span>
        </div>
      `).join("") : `<div class="empty">Canonical validation produced no diagnostics.</div>`}
    </div>
  `;
}

function renderVerify() {
  const module = currentModule();
  if (!module) return `<div class="empty">Select a module to inspect verification evidence.</div>`;
  const query = lower(state.query.trim());
  const proofNodes = module.atlas.nodes
    .filter((node) => node.layer === "verification" && nodeMatches(node, query))
    .sort((left, right) => right.emphasis - left.emphasis || left.label.localeCompare(right.label));
  const constraintNodes = module.atlas.nodes.filter((node) => node.layer === "constraints");
  const traceGaps = module.atlas.traces.reduce((total, trace) => total + trace.gaps.length, 0);
  return `
    ${heading(`${module.name} proof surface`, "Executable evidence remains connected to the promises, operations, traces, and source references it supports.", module.manifest_path)}
    <div class="stats">
      ${stat(constraintNodes.length, "Constraints")}
      ${stat(proofNodes.length, "Evidence")}
      ${stat(module.atlas.traces.length, "Traces")}
      ${stat(traceGaps, "Trace gaps")}
      ${stat(state.snapshot.diagnostics.length, "Diagnostics")}
    </div>
    <div class="semantic-list">
      ${proofNodes.length ? proofNodes.map((node) => semanticRow(module, node)).join("") : `<div class="empty">No verification nodes match the current search.</div>`}
    </div>
  `;
}

function semanticRow(module, node) {
  const active = state.selectedNode?.moduleId === module.id && state.selectedNode?.nodeId === node.id;
  return `
    <button class="semantic-row ${active ? "active" : ""}" type="button" data-module="${escapeHtml(module.id)}" data-node="${escapeHtml(node.id)}">
      <span class="semantic-title">${escapeHtml(node.label)}</span>
      <span class="semantic-summary">${escapeHtml(node.summary)}</span>
      <span class="semantic-meta">${escapeHtml(node.kind)} &middot; ${escapeHtml(node.layer)}</span>
    </button>
  `;
}

function renderInspector() {
  const selected = selectedNodeEntry();
  if (selected) {
    renderNodeInspector(selected.module, selected.node);
    return;
  }
  const module = currentModule();
  if (!module) {
    elements.inspectorKind.textContent = "System";
    elements.inspector.innerHTML = `<div class="empty">Select a module or semantic object.</div>`;
    return;
  }
  elements.inspectorKind.textContent = module.kind;
  const layers = countBy(module.atlas.nodes, (node) => node.layer);
  const publicNodes = module.atlas.nodes.filter((node) => node.layer === "public-surface").slice(0, 8);
  elements.inspector.innerHTML = `
    <h2>${escapeHtml(module.name)}</h2>
    <div class="inspector-meta"><span class="badge">${escapeHtml(module.kind)}</span><span class="badge">${module.atlas.nodes.length} semantics</span></div>
    <p>${escapeHtml(module.purpose)}</p>
    <h3>Source</h3>
    ${sourceButtons([{ role: "module-manifest", path: module.manifest_path }])}
    <h3>Semantic layers</h3>
    <div class="clause-list">${Object.entries(layers).map(([layer, count]) => `<div class="clause"><strong>${escapeHtml(layer)}</strong><span>${count} objects</span></div>`).join("")}</div>
    <h3>Public surface</h3>
    <div class="neighbor-list">${publicNodes.length ? publicNodes.map((node) => `<button class="neighbor-button" type="button" data-module="${escapeHtml(module.id)}" data-node="${escapeHtml(node.id)}">${escapeHtml(node.label)}</button>`).join("") : `<p>No public surface nodes declared.</p>`}</div>
  `;
}

function renderNodeInspector(module, node) {
  elements.inspectorKind.textContent = node.kind;
  const neighbors = nodeNeighbors(module, node);
  elements.inspector.innerHTML = `
    <h2>${escapeHtml(node.label)}</h2>
    <div class="inspector-meta"><span class="badge">${escapeHtml(node.kind)}</span><span class="badge">${escapeHtml(node.layer)}</span><span class="badge">emphasis ${escapeHtml(node.emphasis)}</span></div>
    <p>${escapeHtml(node.summary)}</p>
    <h3>Source references</h3>
    ${sourceButtons(node.source_refs ?? [])}
    <h3>Contract clauses</h3>
    <div class="clause-list">${(node.clauses ?? []).length ? node.clauses.map((clause) => `<div class="clause"><strong>${escapeHtml(clause.label)}</strong><span>${escapeHtml(clause.statement)}</span></div>`).join("") : `<p>No directly attached clauses.</p>`}</div>
    <h3>Semantic neighborhood</h3>
    <div class="neighbor-list">${neighbors.length ? neighbors.map((neighbor) => `<button class="neighbor-button" type="button" data-module="${escapeHtml(module.id)}" data-node="${escapeHtml(neighbor.id)}">${escapeHtml(neighbor.label)} &middot; ${escapeHtml(neighbor.kind)}</button>`).join("") : `<p>No directly declared neighbors.</p>`}</div>
  `;
}

function sourceButtons(sourceRefs) {
  if (!sourceRefs.length) return `<p>No source reference declared.</p>`;
  return `<div class="source-list">${sourceRefs.map((sourceRef) => `<button class="source-button" type="button" data-copy="${escapeHtml(sourceRef.path)}" title="Copy source path">${escapeHtml(sourceRef.path)}</button>`).join("")}</div>`;
}

function nodeNeighbors(module, node) {
  const ids = new Set();
  for (const edge of module.atlas.edges) {
    if (edge.from === node.id) ids.add(edge.to);
    if (edge.to === node.id) ids.add(edge.from);
  }
  return [...ids].map((id) => module.atlas.nodes.find((candidate) => candidate.id === id)).filter(Boolean).slice(0, 12);
}

function nodeLabel(module, nodeId) {
  return module.atlas.nodes.find((node) => node.id === nodeId)?.label ?? nodeId;
}

function countBy(items, key) {
  return items.reduce((counts, item) => {
    const value = key(item);
    counts[value] = (counts[value] ?? 0) + 1;
    return counts;
  }, {});
}

function renderStatus() {
  const { source } = state.snapshot;
  const changed = state.diff.added + state.diff.changed + state.diff.removed;
  elements.statusMessage.textContent = changed
    ? "Canonical semantics changed since the previous snapshot"
    : source.authority;
  elements.diff.innerHTML = `
    <span class="added">+${state.diff.added}</span>
    <span class="changed">~${state.diff.changed}</span>
    <span class="removed">-${state.diff.removed}</span>
  `;
  elements.revision.textContent = source.source_revision ?? "uncommitted source";
}

function showToast(message) {
  elements.toast.textContent = message;
  elements.toast.classList.add("visible");
  window.clearTimeout(showToast.timer);
  showToast.timer = window.setTimeout(() => elements.toast.classList.remove("visible"), 1600);
}

document.addEventListener("click", async (event) => {
  const modeButton = event.target.closest("[data-mode]");
  if (modeButton) {
    state.mode = modeButton.dataset.mode;
    state.selectedNode = null;
    render();
    return;
  }
  const traceButton = event.target.closest("[data-trace]");
  if (traceButton) {
    state.selectedTrace = traceButton.dataset.trace;
    render();
    return;
  }
  const semanticButton = event.target.closest("[data-node]");
  if (semanticButton) {
    state.selectedModule = semanticButton.dataset.module;
    state.selectedNode = { moduleId: semanticButton.dataset.module, nodeId: semanticButton.dataset.node };
    render();
    return;
  }
  const moduleButton = event.target.closest("[data-module]");
  if (moduleButton) {
    state.selectedModule = moduleButton.dataset.module;
    state.selectedNode = null;
    state.selectedTrace = null;
    render();
    return;
  }
  const copyButton = event.target.closest("[data-copy]");
  if (copyButton) {
    await navigator.clipboard.writeText(copyButton.dataset.copy);
    showToast(`Copied ${copyButton.dataset.copy}`);
  }
});

elements.search.addEventListener("input", (event) => {
  state.query = event.target.value;
  renderModules();
  renderStage();
});

elements.refresh.addEventListener("click", () => loadSnapshot(true));

loadSnapshot(false);

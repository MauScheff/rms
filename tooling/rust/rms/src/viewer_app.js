"use strict";

(function bootstrap(global) {
  const VIEW_DEFINITIONS = [
    { id: "system", label: "System map", description: "Module ownership, composition, exports, and dependencies." },
    { id: "behaviors", label: "Behavior paths", description: "Public contracts and required capabilities through exact bindings." },
    { id: "machines", label: "Machines", description: "States, classified inputs, transition cases, outputs, and effects." },
    { id: "proofs", label: "Proofs", description: "Promises, semantic owners, evidence, and replay coverage." },
    { id: "gaps", label: "Gap triage", description: "Required gaps, unresolved links, recommendations, and applicability." },
    { id: "debug", label: "Debug timeline", description: "Execution-derived transition records and source provenance." },
  ];
  const STATUS_ORDER = ["required-gap", "unresolved-link", "recommendation", "satisfied", "not-applicable"];
  const BEHAVIOR_KINDS = new Set(["public-command", "public-query", "public-event", "public-capability", "required-capability"]);
  const PATH_KINDS = new Set([
    ...BEHAVIOR_KINDS,
    "public-behavior-binding", "dependency-behavior-binding", "semantic-function",
    "command", "observed-event", "effect-result", "transition-case", "state",
    "event", "effect", "reply", "rejection", "invariant", "evidence",
    "trace-bundle", "trace-record", "machine",
  ]);

  function compare(a, b) {
    return String(a.label ?? a.id).localeCompare(String(b.label ?? b.id)) || String(a.id).localeCompare(String(b.id));
  }

  function text(value) {
    return String(value ?? "").toLocaleLowerCase();
  }

  function addToMapList(map, key, value) {
    if (!map.has(key)) map.set(key, []);
    map.get(key).push(value);
  }

  function groupBy(items, selector) {
    const groups = Object.create(null);
    for (const item of items) {
      const key = String(selector(item));
      if (!groups[key]) groups[key] = [];
      groups[key].push(item);
    }
    return groups;
  }

  function buildIndex(snapshot) {
    const graph = snapshot.graph ?? { nodes: [], edges: [], obligations: [] };
    const nodesById = new Map(graph.nodes.map((node) => [node.id, node]));
    const edgesById = new Map(graph.edges.map((edge) => [edge.id, edge]));
    const obligationsById = new Map(graph.obligations.map((item) => [item.id, item]));
    const outgoing = new Map();
    const incoming = new Map();
    const nodesByModule = new Map();
    const obligationsByModule = new Map();
    for (const edge of graph.edges) {
      addToMapList(outgoing, edge.from, edge);
      addToMapList(incoming, edge.to, edge);
    }
    for (const node of graph.nodes) addToMapList(nodesByModule, node.module_id, node);
    for (const obligation of graph.obligations) addToMapList(obligationsByModule, obligation.module_id, obligation);
    for (const values of [...outgoing.values(), ...incoming.values()]) values.sort((a, b) => a.id.localeCompare(b.id));
    for (const values of nodesByModule.values()) values.sort(compare);
    for (const values of obligationsByModule.values()) values.sort((a, b) => {
      return STATUS_ORDER.indexOf(a.status) - STATUS_ORDER.indexOf(b.status) || a.id.localeCompare(b.id);
    });
    return { snapshot, graph, nodesById, edgesById, obligationsById, outgoing, incoming, nodesByModule, obligationsByModule };
  }

  function matchesQuery(value, query) {
    if (!query) return true;
    const refs = (value.source_refs ?? []).map((source) => `${source.role} ${source.path}`).join(" ");
    const details = Object.entries(value.details ?? {}).flat().join(" ");
    const lists = Object.values(value.lists ?? {}).flat().join(" ");
    return text(`${value.id} ${value.kind} ${value.label} ${value.summary} ${value.title} ${value.detail} ${refs} ${details} ${lists}`).includes(text(query));
  }

  function nodes(index, options = {}) {
    const kinds = options.kinds ? new Set(options.kinds) : null;
    return index.graph.nodes.filter((node) => {
      if (kinds && !kinds.has(node.kind)) return false;
      if (options.moduleId && node.module_id !== options.moduleId) return false;
      return matchesQuery(node, options.query);
    }).sort(compare);
  }

  function obligations(index, options = {}) {
    return index.graph.obligations.filter((item) => {
      if (options.moduleId && item.module_id !== options.moduleId) return false;
      if (options.status && options.status !== "all" && item.status !== options.status) return false;
      return matchesQuery(item, options.query);
    }).sort((a, b) => {
      return STATUS_ORDER.indexOf(a.status) - STATUS_ORDER.indexOf(b.status) || a.title.localeCompare(b.title) || a.id.localeCompare(b.id);
    });
  }

  function statusCounts(items) {
    const counts = Object.fromEntries(STATUS_ORDER.map((status) => [status, 0]));
    for (const item of items) counts[item.status] = (counts[item.status] ?? 0) + 1;
    return counts;
  }

  function moduleStatus(index, moduleId) {
    const items = index.obligationsByModule.get(moduleId) ?? [];
    return STATUS_ORDER.find((status) => items.some((item) => item.status === status)) ?? "not-applicable";
  }

  function semanticFingerprint(snapshot) {
    const values = new Map();
    for (const node of snapshot?.graph?.nodes ?? []) values.set(`node:${node.id}`, JSON.stringify(node));
    for (const edge of snapshot?.graph?.edges ?? []) values.set(`edge:${edge.id}`, JSON.stringify(edge));
    for (const item of snapshot?.graph?.obligations ?? []) values.set(`obligation:${item.id}`, JSON.stringify(item));
    return values;
  }

  function semanticDiff(previous, next) {
    if (!previous) return { added: 0, changed: 0, removed: 0, unresolved: 0 };
    const before = semanticFingerprint(previous);
    const after = semanticFingerprint(next);
    let added = 0;
    let changed = 0;
    let removed = 0;
    for (const [id, value] of after) {
      if (!before.has(id)) added += 1;
      else if (before.get(id) !== value) changed += 1;
    }
    for (const id of before.keys()) if (!after.has(id)) removed += 1;
    const unresolved = (next.graph?.obligations ?? []).filter((item) => item.status === "unresolved-link").length;
    return { added, changed, removed, unresolved };
  }

  function neighborhood(index, startId, options = {}) {
    if (!index.nodesById.has(startId)) return [];
    const maxDepth = options.maxDepth ?? 5;
    const maxNodes = options.maxNodes ?? 90;
    const allowedKinds = options.allowedKinds ?? PATH_KINDS;
    const visited = new Set([startId]);
    const queue = [{ nodeId: startId, depth: 0, via: null, previous: null }];
    const result = [];
    while (queue.length && result.length < maxNodes) {
      const current = queue.shift();
      const node = index.nodesById.get(current.nodeId);
      if (!node) continue;
      result.push({ ...current, node });
      if (current.depth >= maxDepth) continue;
      const connections = [
        ...(index.outgoing.get(current.nodeId) ?? []).map((edge) => ({ edge, next: edge.to, direction: "out" })),
        ...(index.incoming.get(current.nodeId) ?? []).map((edge) => ({ edge, next: edge.from, direction: "in" })),
      ].sort((a, b) => a.edge.id.localeCompare(b.edge.id));
      for (const connection of connections) {
        if (visited.has(connection.next)) continue;
        const nextNode = index.nodesById.get(connection.next);
        if (!nextNode || !allowedKinds.has(nextNode.kind)) continue;
        if (options.sameModule && nextNode.module_id !== node.module_id && nextNode.kind !== "module") continue;
        visited.add(connection.next);
        queue.push({
          nodeId: connection.next,
          depth: current.depth + 1,
          via: { ...connection.edge, direction: connection.direction },
          previous: current.nodeId,
        });
      }
    }
    return result;
  }

  function systemRelationships(index) {
    const relationships = [];
    const seen = new Set();
    for (const edge of index.graph.edges) {
      const from = index.nodesById.get(edge.from);
      const to = index.nodesById.get(edge.to);
      if (!from || !to) continue;
      let fromModule = null;
      let toModule = null;
      if (["contains", "requires-module"].includes(edge.kind) && from.kind === "module" && to.kind === "module") {
        fromModule = from.id;
        toModule = to.id;
      } else if (edge.kind === "exports" && from.kind === "module") {
        fromModule = from.id;
        toModule = to.module_id;
      } else if (edge.kind === "delegates-to" && from.kind === "dependency-behavior-binding" && to.kind === "module") {
        fromModule = from.module_id;
        toModule = to.id;
      }
      if (!fromModule || !toModule) continue;
      const key = `${edge.kind}:${fromModule}:${toModule}:${edge.label}`;
      if (seen.has(key)) continue;
      seen.add(key);
      relationships.push({ ...edge, fromModule, toModule });
    }
    return relationships.sort((a, b) => a.id.localeCompare(b.id));
  }

  function traceRecords(index, bundleId) {
    return (index.outgoing.get(bundleId) ?? [])
      .filter((edge) => edge.kind === "contains")
      .map((edge) => index.nodesById.get(edge.to))
      .filter((node) => node?.kind === "trace-record")
      .sort((a, b) => a.label.localeCompare(b.label, undefined, { numeric: true }));
  }

  function parseUrl(locationLike) {
    const params = new URLSearchParams(locationLike.search ?? "");
    const view = VIEW_DEFINITIONS.some((item) => item.id === params.get("view")) ? params.get("view") : "system";
    return {
      view,
      nodeId: params.get("node"),
      edgeId: params.get("edge"),
      obligationId: params.get("obligation"),
      moduleId: params.get("module"),
      status: STATUS_ORDER.includes(params.get("status")) ? params.get("status") : "all",
      query: params.get("q") ?? "",
    };
  }

  function urlFor(state, locationLike) {
    const params = new URLSearchParams();
    if (state.view && state.view !== "system") params.set("view", state.view);
    if (state.nodeId) params.set("node", state.nodeId);
    if (state.edgeId) params.set("edge", state.edgeId);
    if (state.obligationId) params.set("obligation", state.obligationId);
    if (state.moduleId) params.set("module", state.moduleId);
    if (state.status && state.status !== "all") params.set("status", state.status);
    if (state.query) params.set("q", state.query);
    const query = params.toString();
    return `${locationLike.pathname ?? "/"}${query ? `?${query}` : ""}`;
  }

  const model = {
    VIEW_DEFINITIONS, STATUS_ORDER, BEHAVIOR_KINDS, PATH_KINDS,
    buildIndex, matchesQuery, nodes, obligations, statusCounts, moduleStatus, groupBy,
    semanticDiff, neighborhood, systemRelationships, traceRecords, parseUrl, urlFor,
  };
  global.RMSViewerModel = model;
  if (typeof module !== "undefined" && module.exports) module.exports = model;
  if (typeof document !== "undefined") initializeApplication(model);
})(typeof globalThis !== "undefined" ? globalThis : this);

function initializeApplication(model) {
  const state = {
    snapshot: null,
    index: null,
    view: "system",
    nodeId: null,
    edgeId: null,
    obligationId: null,
    moduleId: null,
    status: "all",
    query: "",
    loading: false,
    pollTimer: null,
    hydrated: false,
    diff: { added: 0, changed: 0, removed: 0, unresolved: 0 },
  };
  const elements = {
    systemName: document.querySelector("#system-name"),
    systemPurpose: document.querySelector("#system-purpose"),
    search: document.querySelector("#search"),
    refresh: document.querySelector("#refresh"),
    liveState: document.querySelector("#live-state"),
    liveLabel: document.querySelector("#live-label"),
    graphCount: document.querySelector("#graph-count"),
    navigation: document.querySelector("#navigation"),
    breadcrumbs: document.querySelector("#breadcrumbs"),
    stage: document.querySelector("#stage"),
    inspector: document.querySelector("#inspector"),
    inspectorPane: document.querySelector("#inspector-pane"),
    inspectorKind: document.querySelector("#inspector-kind"),
    inspectorClose: document.querySelector("#inspector-close"),
    navToggle: document.querySelector("#nav-toggle"),
    leftPane: document.querySelector("#left-pane"),
    scrim: document.querySelector("#scrim"),
    statusMessage: document.querySelector("#status-message"),
    semanticDiff: document.querySelector("#semantic-diff"),
    revision: document.querySelector("#revision"),
    toast: document.querySelector("#toast"),
    main: document.querySelector("#main"),
  };

  function escapeHtml(value) {
    return String(value ?? "")
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#039;");
  }

  function currentModuleNode() {
    return state.moduleId ? state.index?.nodesById.get(state.moduleId) ?? null : null;
  }

  function selectedNode() {
    return state.nodeId ? state.index?.nodesById.get(state.nodeId) ?? null : null;
  }

  function selectedEdge() {
    return state.edgeId ? state.index?.edgesById.get(state.edgeId) ?? null : null;
  }

  function selectedObligation() {
    return state.obligationId ? state.index?.obligationsById.get(state.obligationId) ?? null : null;
  }

  function applyUrlState() {
    const urlState = model.parseUrl(window.location);
    Object.assign(state, urlState);
    elements.search.value = state.query;
  }

  function writeUrl(mode = "push") {
    const url = model.urlFor(state, window.location);
    const method = mode === "replace" ? "replaceState" : "pushState";
    window.history[method]({}, "", url);
  }

  function normalizeSelection() {
    let removed = null;
    if (state.nodeId && !state.index.nodesById.has(state.nodeId)) {
      removed = state.nodeId;
      state.nodeId = null;
    }
    if (state.edgeId && !state.index.edgesById.has(state.edgeId)) {
      removed = state.edgeId;
      state.edgeId = null;
    }
    if (state.obligationId && !state.index.obligationsById.has(state.obligationId)) {
      removed = state.obligationId;
      state.obligationId = null;
    }
    if (state.moduleId && !state.index.nodesById.has(state.moduleId)) state.moduleId = null;
    const node = selectedNode();
    if (node) state.moduleId = node.module_id;
    const obligation = selectedObligation();
    if (obligation) state.moduleId = obligation.module_id;
    if (!state.moduleId) state.moduleId = model.nodes(state.index, { kinds: ["module"] })[0]?.id ?? null;
    if (removed) showToast(`The selected semantic object was removed: ${removed}`);
  }

  async function loadSnapshot(manual = false) {
    if (state.loading) return;
    state.loading = true;
    if (manual) elements.statusMessage.textContent = "Refreshing canonical graph...";
    try {
      const response = await fetch("/api/snapshot", { cache: "no-store" });
      if (!response.ok) throw new Error(`snapshot request failed (${response.status})`);
      const next = await response.json();
      state.diff = model.semanticDiff(state.snapshot, next);
      state.snapshot = next;
      state.index = model.buildIndex(next);
      const firstHydration = !state.hydrated;
      if (!state.hydrated) {
        applyUrlState();
        state.hydrated = true;
      }
      normalizeSelection();
      render();
      if (firstHydration && (state.nodeId || state.edgeId || state.obligationId)) {
        openInspectorOnCompactScreens();
      }
      configurePolling();
    } catch (error) {
      elements.stage.innerHTML = `<div class="empty">${escapeHtml(error.message)}</div>`;
      elements.statusMessage.textContent = "Canonical projection unavailable";
      elements.liveState.classList.remove("active");
      elements.liveLabel.textContent = "Offline";
    } finally {
      state.loading = false;
    }
  }

  function configurePolling() {
    if (state.pollTimer || !state.snapshot?.source?.refresh_ms) return;
    state.pollTimer = window.setInterval(() => loadSnapshot(false), state.snapshot.source.refresh_ms);
  }

  function render() {
    if (!state.index) return;
    renderHeader();
    renderNavigation();
    renderBreadcrumbs();
    renderStage();
    renderInspector();
    renderStatus();
  }

  function renderHeader() {
    elements.systemName.textContent = state.snapshot.system.name;
    elements.systemPurpose.textContent = state.snapshot.system.purpose ?? "";
    const live = state.snapshot.source.refresh_ms > 0;
    elements.liveState.classList.toggle("active", live);
    elements.liveLabel.textContent = live ? "Live" : "Snapshot";
    elements.graphCount.textContent = `${state.index.graph.nodes.length} objects`;
  }

  function viewCount(viewId) {
    const all = state.index.graph.nodes;
    if (viewId === "system") return all.filter((node) => node.kind === "module").length;
    if (viewId === "behaviors") return all.filter((node) => model.BEHAVIOR_KINDS.has(node.kind)).length;
    if (viewId === "machines") return all.filter((node) => node.kind === "machine").length;
    if (viewId === "proofs") return all.filter((node) => node.kind === "invariant").length;
    if (viewId === "gaps") return state.index.graph.obligations.filter((item) => !["satisfied", "not-applicable"].includes(item.status)).length;
    return all.filter((node) => node.kind === "trace-record").length;
  }

  function renderNavigation() {
    const modules = model.nodes(state.index, { kinds: ["module"], query: state.query });
    elements.navigation.innerHTML = `
      <div class="nav-group-title">Views</div>
      ${model.VIEW_DEFINITIONS.map((view) => `
        <button class="view-button ${view.id === state.view ? "active" : ""}" type="button" data-view="${view.id}" title="${escapeHtml(view.description)}">
          <span class="view-label">${escapeHtml(view.label)}</span><span class="nav-count">${viewCount(view.id)}</span>
        </button>
      `).join("")}
      <div class="nav-group-title">Module scope</div>
      ${modules.map((module) => `
        <button class="scope-button ${module.id === state.moduleId ? "active" : ""}" type="button" data-module="${escapeHtml(module.id)}">
          <span class="view-label">${escapeHtml(module.label)}<span class="scope-meta">${escapeHtml(module.details?.shape || module.details?.kind || "module")}</span></span>
          <span class="status ${model.moduleStatus(state.index, module.id)}" title="${model.moduleStatus(state.index, module.id)}"></span>
        </button>
      `).join("") || `<div class="empty">No modules match.</div>`}
    `;
  }

  function renderBreadcrumbs() {
    const view = model.VIEW_DEFINITIONS.find((item) => item.id === state.view);
    const module = currentModuleNode();
    const entity = selectedNode() ?? selectedObligation();
    elements.breadcrumbs.innerHTML = `
      <button class="crumb-button" type="button" data-view="system">${escapeHtml(state.snapshot.system.name)}</button>
      <span>/</span><button class="crumb-button" type="button" data-view="${escapeHtml(state.view)}">${escapeHtml(view?.label ?? state.view)}</button>
      ${module ? `<span>/</span><button class="crumb-button" type="button" data-module="${escapeHtml(module.id)}">${escapeHtml(module.label)}</button>` : ""}
      ${entity ? `<span>/</span><span>${escapeHtml(entity.label ?? entity.title)}</span>` : ""}
    `;
  }

  function viewHead(title, description, tools = "") {
    return `<div class="view-head"><div><h2>${escapeHtml(title)}</h2><p>${escapeHtml(description)}</p></div><div class="view-tools">${tools}</div></div>`;
  }

  function metric(value, label) {
    return `<div class="metric"><strong>${escapeHtml(value)}</strong><span>${escapeHtml(label)}</span></div>`;
  }

  function renderStage() {
    const renderers = { system: renderSystem, behaviors: renderBehaviors, machines: renderMachines, proofs: renderProofs, gaps: renderGaps, debug: renderDebug };
    elements.stage.innerHTML = (renderers[state.view] ?? renderSystem)();
  }

  function renderSystem() {
    const modules = model.nodes(state.index, { kinds: ["module"], query: state.query });
    const relationships = model.systemRelationships(state.index).filter((edge) => {
      if (!state.query) return true;
      const from = state.index.nodesById.get(edge.fromModule);
      const to = state.index.nodesById.get(edge.toModule);
      return model.matchesQuery(from ?? {}, state.query) || model.matchesQuery(to ?? {}, state.query) || model.matchesQuery(edge, state.query);
    });
    const counts = model.statusCounts(state.index.graph.obligations);
    return `
      ${viewHead("System map", "Ownership, composition, exports, and dependency direction from canonical graph edges.")}
      <div class="metric-strip">
        ${metric(modules.length, "Modules")}${metric(relationships.length, "Relationships")}${metric(state.index.graph.nodes.length, "Semantic objects")}${metric(counts["required-gap"] + counts["unresolved-link"], "Blocking findings")}${metric(counts["not-applicable"], "Not applicable")}
      </div>
      <div class="section-title">Modules</div>
      <div class="module-grid">${modules.map(renderModuleNode).join("") || `<div class="empty">No modules match the current search.</div>`}</div>
      <div class="section-title">Declared relationships</div>
      <div class="relationship-table">${relationships.map((edge) => {
        const from = state.index.nodesById.get(edge.fromModule);
        const to = state.index.nodesById.get(edge.toModule);
        return `<button class="relationship-row ${state.edgeId === edge.id ? "active" : ""}" type="button" data-edge="${escapeHtml(edge.id)}"><span class="row-primary">${escapeHtml(from?.label ?? edge.fromModule)}</span><span class="tag">${escapeHtml(edge.kind)}</span><span class="row-primary">${escapeHtml(to?.label ?? edge.toModule)}</span><span class="row-secondary">${escapeHtml(edge.label)}</span></button>`;
      }).join("") || `<div class="empty">No cross-module relationships are declared.</div>`}</div>
    `;
  }

  function renderModuleNode(module) {
    const moduleNodes = state.index.nodesByModule.get(module.id) ?? [];
    const obligations = state.index.obligationsByModule.get(module.id) ?? [];
    const counts = model.statusCounts(obligations);
    return `<button class="module-node ${escapeHtml(module.details?.shape ?? "")} ${module.id === state.moduleId ? "active" : ""}" type="button" data-module="${escapeHtml(module.id)}"><span class="tag">${escapeHtml(module.details?.shape || module.details?.kind || "module")}</span><h3>${escapeHtml(module.label)}</h3><p>${escapeHtml(module.summary)}</p><span class="module-foot"><span>${moduleNodes.length} objects</span><span>${counts["required-gap"] + counts["unresolved-link"]} gaps</span><span>${counts.satisfied} satisfied</span></span></button>`;
  }

  function behaviorRoots() {
    return model.nodes(state.index, { kinds: [...model.BEHAVIOR_KINDS], moduleId: state.moduleId, query: state.query });
  }

  function renderBehaviors() {
    let roots = behaviorRoots();
    if (!roots.length) roots = model.nodes(state.index, { kinds: [...model.BEHAVIOR_KINDS], query: state.query });
    const current = selectedNode();
    const root = current && model.BEHAVIOR_KINDS.has(current.kind) ? current : roots[0];
    const path = root ? model.neighborhood(state.index, root.id, { maxDepth: 5, maxNodes: 80 }) : [];
    const levels = model.groupBy(path, (item) => item.depth);
    return `
      ${viewHead("Behavior paths", "Follow a public promise or required capability through exact bindings, machine cases, effects, and proof.")}
      <div class="path-layout">
        <div class="path-roots">${roots.map((node) => `<button class="path-root ${root?.id === node.id ? "active" : ""}" type="button" data-node="${escapeHtml(node.id)}"><strong>${escapeHtml(node.label)}</strong><span>${escapeHtml(node.kind)} · ${escapeHtml(state.index.nodesById.get(node.module_id)?.label ?? node.module_id)}</span></button>`).join("") || `<div class="empty">No behavior matches this scope.</div>`}</div>
        <div class="path-canvas">${root ? `<div class="path-levels">${Object.entries(levels).map(([depth, items]) => `<section class="path-level"><div class="path-level-title">${depth === "0" ? "Public intent" : `Declared step ${depth}`}</div>${items.map(renderPathNode).join("")}</section>`).join("")}</div>` : `<div class="empty">Select a public behavior or required capability.</div>`}</div>
      </div>
    `;
  }

  function renderPathNode(item) {
    const edge = item.via;
    const direction = edge ? (edge.direction === "out" ? "→" : "←") : "";
    return `<button class="path-node ${state.nodeId === item.node.id ? "active" : ""}" type="button" data-node="${escapeHtml(item.node.id)}">${edge ? `<span class="edge-label">${direction} ${escapeHtml(edge.label)}</span>` : ""}<strong>${escapeHtml(item.node.label)}</strong><span>${escapeHtml(item.node.kind)} · ${escapeHtml(item.node.summary)}</span></button>`;
  }

  function renderMachines() {
    let machines = model.nodes(state.index, { kinds: ["machine"], moduleId: state.moduleId, query: state.query });
    if (!machines.length) machines = model.nodes(state.index, { kinds: ["machine"], query: state.query });
    const selected = selectedNode();
    const machine = selected?.kind === "machine" ? selected : machines[0];
    if (!machine) return `${viewHead("Machines", "States, classified inputs, transition cases, and outputs.")}<div class="empty">No implemented machine matches this scope.</div>`;
    const moduleNodes = state.index.nodesByModule.get(machine.module_id) ?? [];
    const states = moduleNodes.filter((node) => node.kind === "state");
    const transitions = moduleNodes.filter((node) => node.kind === "transition-case").filter((node) => model.matchesQuery(node, state.query)).sort(compareNodes);
    const inputKinds = ["command", "observed-event", "effect-result"];
    const terminal = new Set(machine.lists?.terminal_states ?? []);
    const tools = machines.length > 1 ? `<select class="select" data-select-machine aria-label="Select machine">${machines.map((item) => `<option value="${escapeHtml(item.id)}" ${item.id === machine.id ? "selected" : ""}>${escapeHtml(item.label)}</option>`).join("")}</select>` : "";
    return `
      ${viewHead(machine.label, machine.summary, tools)}
      <div class="machine-summary">
        <div><div class="section-title">States</div><div class="state-strip">${states.map((node) => `<button class="state-node ${node.label === machine.details?.initial_state ? "initial" : ""} ${terminal.has(node.label) ? "terminal" : ""}" type="button" data-node="${escapeHtml(node.id)}"><strong>${escapeHtml(node.label)}</strong><span>${node.label === machine.details?.initial_state ? "initial" : terminal.has(node.label) ? "terminal" : "state"}</span></button>`).join("") || `<span class="tag">Unit state</span>`}</div></div>
        <div class="input-groups">${inputKinds.map((kind) => { const items = moduleNodes.filter((node) => node.kind === kind); return `<div class="input-group"><strong>${escapeHtml(kind.replaceAll("-", " "))}</strong><div class="input-tags">${items.map((node) => `<button class="tag" type="button" data-node="${escapeHtml(node.id)}">${escapeHtml(node.label)}</button>`).join("") || `<span class="tag">none</span>`}</div></div>`; }).join("")}</div>
      </div>
      <div class="section-title">Canonical transition cases</div>
      <div class="transition-table">${transitions.map((transition) => {
        const outputs = [
          ...(transition.lists?.events ?? []), ...(transition.lists?.commands ?? []), ...(transition.lists?.effects ?? []),
          transition.details?.reply, transition.details?.rejection,
        ].filter(Boolean).join(", ");
        return `<button class="transition-row ${state.nodeId === transition.id ? "active" : ""}" type="button" data-node="${escapeHtml(transition.id)}"><span class="row-primary">${escapeHtml(transition.label)}</span><span class="row-secondary">${escapeHtml(transition.details?.from || "?")}</span><span class="row-secondary">${escapeHtml(transition.details?.on || "?")}</span><span class="row-secondary">${escapeHtml(transition.details?.to || "?")}</span><span class="row-meta">${escapeHtml(outputs || "no emitted output")}</span></button>`;
      }).join("") || `<div class="empty">No transition cases match.</div>`}</div>
    `;
  }

  function compareNodes(a, b) { return String(a.label).localeCompare(String(b.label)) || a.id.localeCompare(b.id); }

  function renderProofs() {
    let promises = model.nodes(state.index, { kinds: ["invariant", "public-command", "public-query", "public-capability"], moduleId: state.moduleId, query: state.query });
    if (!promises.length) promises = model.nodes(state.index, { kinds: ["invariant", "public-command", "public-query", "public-capability"], query: state.query });
    const selected = selectedNode();
    const promise = selected && ["invariant", "public-command", "public-query", "public-capability"].includes(selected.kind) ? selected : promises[0];
    const chain = promise ? model.neighborhood(state.index, promise.id, { maxDepth: 4, maxNodes: 70 }) : [];
    return `
      ${viewHead("Proof chains", "Promises first: authority, semantic owner, executable evidence, traces, and exact missing links.")}
      <div class="proof-grid">
        <div class="promise-list">${promises.map((node) => `<button class="path-root ${promise?.id === node.id ? "active" : ""}" type="button" data-node="${escapeHtml(node.id)}"><strong>${escapeHtml(node.label)}</strong><span>${escapeHtml(node.kind)} · ${escapeHtml(node.summary)}</span></button>`).join("") || `<div class="empty">No promises match.</div>`}</div>
        <div class="proof-chain">${chain.length ? chain.map((item) => `<div class="proof-step"><button type="button" data-node="${escapeHtml(item.node.id)}"><strong>${escapeHtml(item.node.label)}</strong></button><p>${item.via ? `${item.via.direction === "out" ? "→" : "←"} ${escapeHtml(item.via.label)} · ` : ""}${escapeHtml(item.node.kind)} · ${escapeHtml(item.node.summary)}</p></div>`).join("") : `<div class="empty">Select a law or public contract.</div>`}</div>
      </div>
    `;
  }

  function renderGaps() {
    const items = model.obligations(state.index, { moduleId: state.moduleId, status: state.status, query: state.query });
    const allScoped = model.obligations(state.index, { moduleId: state.moduleId });
    const counts = model.statusCounts(allScoped);
    const tools = `<select class="select" data-status-filter aria-label="Filter findings by status"><option value="all">All statuses</option>${model.STATUS_ORDER.map((status) => `<option value="${status}" ${state.status === status ? "selected" : ""}>${status} (${counts[status] ?? 0})</option>`).join("")}</select>`;
    return `
      ${viewHead("Gap triage", "Applicability stays explicit: missing, unresolved, recommended, satisfied, and not-applicable are different facts.", tools)}
      <div class="metric-strip">${metric(counts["required-gap"], "Required gaps")}${metric(counts["unresolved-link"], "Unresolved links")}${metric(counts.recommendation, "Recommendations")}${metric(counts.satisfied, "Satisfied")}${metric(counts["not-applicable"], "Not applicable")}</div>
      <div class="finding-list">${items.map((item) => `<button class="finding-row ${state.obligationId === item.id ? "active" : ""}" type="button" data-obligation="${escapeHtml(item.id)}"><span class="status ${escapeHtml(item.status)}">${escapeHtml(item.status)}</span><span class="row-primary">${escapeHtml(item.title)}</span><span class="row-secondary">${escapeHtml(item.detail)}</span></button>`).join("") || `<div class="empty">No obligations match this filter.</div>`}</div>
    `;
  }

  function renderDebug() {
    let bundles = model.nodes(state.index, { kinds: ["trace-bundle"], moduleId: state.moduleId, query: state.query });
    if (!bundles.length) bundles = model.nodes(state.index, { kinds: ["trace-bundle"], query: state.query });
    const selected = selectedNode();
    let bundle = selected?.kind === "trace-bundle" ? selected : null;
    if (selected?.kind === "trace-record") {
      const parent = (state.index.incoming.get(selected.id) ?? []).find((edge) => edge.kind === "contains");
      bundle = parent ? state.index.nodesById.get(parent.from) : null;
    }
    bundle ??= bundles[0];
    const records = bundle ? model.traceRecords(state.index, bundle.id).filter((record) => model.matchesQuery(record, state.query)) : [];
    const tools = bundles.length > 1 ? `<select class="select" data-select-trace aria-label="Select trace bundle">${bundles.map((item) => `<option value="${escapeHtml(item.id)}" ${item.id === bundle?.id ? "selected" : ""}>${escapeHtml(state.index.nodesById.get(item.module_id)?.label ?? item.module_id)} · ${escapeHtml(item.label)}</option>`).join("")}</select>` : "";
    return `
      ${viewHead(bundle?.label ?? "Debug timeline", bundle?.summary ?? "Execution-derived records are required for a timeline; no synthetic history is shown.", tools)}
      <div class="timeline">${records.map((record) => {
        const rejected = Boolean(record.details?.rejection);
        const outputs = [...(record.lists?.events ?? []), ...(record.lists?.commands ?? []), ...(record.lists?.effects ?? [])];
        return `<article class="timeline-record ${rejected ? "rejected" : ""}"><h3><button class="crumb-button" type="button" data-node="${escapeHtml(record.id)}">${escapeHtml(record.label)}</button> ${rejected ? `<span class="status required-gap">rejected</span>` : `<span class="status satisfied">accepted</span>`}</h3><p>${escapeHtml(record.details?.input || record.summary)}</p><div class="state-change"><span>${escapeHtml(record.details?.state_before || "unknown before")}</span><span class="arrow">→</span><span>${escapeHtml(record.details?.state_after || "unknown after")}</span></div>${outputs.length ? `<p><strong>Outputs:</strong> ${escapeHtml(outputs.join(", "))}</p>` : ""}${record.details?.reply ? `<p><strong>Reply:</strong> ${escapeHtml(record.details.reply)}</p>` : ""}${record.details?.rejection ? `<p><strong>Rejection:</strong> ${escapeHtml(record.details.rejection)}</p>` : ""}</article>`;
      }).join("") || `<div class="empty">No execution-derived transition records match this scope.</div>`}</div>
    `;
  }

  function renderInspector() {
    const obligation = selectedObligation();
    const edge = selectedEdge();
    const node = selectedNode();
    if (obligation) return renderObligationInspector(obligation);
    if (edge) return renderEdgeInspector(edge);
    if (node) return renderNodeInspector(node);
    const module = currentModuleNode();
    if (module) return renderModuleInspector(module);
    elements.inspectorKind.textContent = "System";
    elements.inspector.innerHTML = `<div class="empty">Select a module, graph object, relationship, or obligation.</div>`;
  }

  function sourceButtons(refs) {
    if (!(refs ?? []).length) return `<p>No source reference declared.</p>`;
    return `<div class="source-list">${refs.map((source) => `<button class="source-button" type="button" data-copy="${escapeHtml(source.path)}" title="Copy source path">${escapeHtml(source.role)} · ${escapeHtml(source.path)}</button>`).join("")}</div>`;
  }

  function detailRows(details, lists = {}) {
    const rows = [...Object.entries(details ?? {}).filter(([, value]) => value), ...Object.entries(lists ?? {}).filter(([, value]) => value?.length).map(([key, value]) => [key, value.join(", ")])];
    return rows.length ? `<dl class="detail-list">${rows.map(([key, value]) => `<div class="detail-row"><dt>${escapeHtml(key.replaceAll("_", " "))}</dt><dd>${escapeHtml(value)}</dd></div>`).join("")}</dl>` : `<p>No additional canonical fields.</p>`;
  }

  function renderNodeInspector(node) {
    elements.inspectorKind.textContent = node.kind;
    const connections = [
      ...(state.index.outgoing.get(node.id) ?? []).map((edge) => ({ edge, neighbor: state.index.nodesById.get(edge.to), direction: "→" })),
      ...(state.index.incoming.get(node.id) ?? []).map((edge) => ({ edge, neighbor: state.index.nodesById.get(edge.from), direction: "←" })),
    ].filter((item) => item.neighbor).sort((a, b) => a.edge.id.localeCompare(b.edge.id));
    elements.inspector.innerHTML = `<h2>${escapeHtml(node.label)}</h2><div class="inspector-tags"><span class="tag">${escapeHtml(node.kind)}</span><span class="tag">${escapeHtml(state.index.nodesById.get(node.module_id)?.label ?? node.module_id)}</span></div><p>${escapeHtml(node.summary)}</p><h3>Canonical fields</h3>${detailRows(node.details, node.lists)}<h3>Why connected</h3><div class="neighbor-list">${connections.map(({ edge, neighbor, direction }) => `<button class="neighbor-button" type="button" data-node="${escapeHtml(neighbor.id)}"><strong>${direction} ${escapeHtml(edge.label)}</strong><br>${escapeHtml(neighbor.label)} · ${escapeHtml(neighbor.kind)}</button>`).join("") || `<p>No declared graph neighbors.</p>`}</div><h3>Source provenance</h3>${sourceButtons(node.source_refs)}`;
  }

  function renderEdgeInspector(edge) {
    elements.inspectorKind.textContent = "relationship";
    const from = state.index.nodesById.get(edge.from);
    const to = state.index.nodesById.get(edge.to);
    elements.inspector.innerHTML = `<h2>${escapeHtml(edge.label)}</h2><div class="inspector-tags"><span class="tag">${escapeHtml(edge.kind)}</span></div><p>This connection exists because the canonical graph declares the edge below.</p><h3>Endpoints</h3><div class="neighbor-list"><button class="neighbor-button" type="button" data-node="${escapeHtml(from?.id ?? edge.from)}"><strong>From</strong><br>${escapeHtml(from?.label ?? edge.from)} · ${escapeHtml(from?.kind ?? "unresolved")}</button><button class="neighbor-button" type="button" data-node="${escapeHtml(to?.id ?? edge.to)}"><strong>To</strong><br>${escapeHtml(to?.label ?? edge.to)} · ${escapeHtml(to?.kind ?? "unresolved")}</button></div><h3>Source provenance</h3>${sourceButtons(edge.source_refs)}`;
  }

  function renderObligationInspector(item) {
    elements.inspectorKind.textContent = "obligation";
    elements.inspector.innerHTML = `<h2>${escapeHtml(item.title)}</h2><div class="inspector-tags"><span class="status ${escapeHtml(item.status)}">${escapeHtml(item.status)}</span><span class="tag">${escapeHtml(item.kind)}</span></div><p>${escapeHtml(item.detail)}</p><h3>Applicability</h3><p>${item.status === "not-applicable" ? "This obligation does not belong to the module's declared shape or behavior." : item.status === "recommendation" ? "This strengthens reliability but is not a production requirement." : item.status === "satisfied" ? "The applicable semantic chain is closed." : "An applicable canonical step is absent or does not resolve."}</p><h3>Source provenance</h3>${sourceButtons(item.source_refs)}`;
  }

  function renderModuleInspector(module) {
    elements.inspectorKind.textContent = "module";
    const moduleNodes = state.index.nodesByModule.get(module.id) ?? [];
    const moduleObligations = state.index.obligationsByModule.get(module.id) ?? [];
    const counts = model.statusCounts(moduleObligations);
    const publicNodes = moduleNodes.filter((node) => model.BEHAVIOR_KINDS.has(node.kind)).slice(0, 10);
    elements.inspector.innerHTML = `<h2>${escapeHtml(module.label)}</h2><div class="inspector-tags"><span class="tag">${escapeHtml(module.details?.shape || module.details?.kind || "module")}</span><span class="status ${model.moduleStatus(state.index, module.id)}">${model.moduleStatus(state.index, module.id)}</span></div><p>${escapeHtml(module.summary)}</p><h3>Semantic inventory</h3>${detailRows({ objects: String(moduleNodes.length), satisfied: String(counts.satisfied), blocking: String(counts["required-gap"] + counts["unresolved-link"]), not_applicable: String(counts["not-applicable"]) })}<h3>Public and required behaviors</h3><div class="neighbor-list">${publicNodes.map((node) => `<button class="neighbor-button" type="button" data-node="${escapeHtml(node.id)}">${escapeHtml(node.label)} · ${escapeHtml(node.kind)}</button>`).join("") || `<p>No public behavior declared.</p>`}</div><h3>Source provenance</h3>${sourceButtons(module.source_refs)}`;
  }

  function renderStatus() {
    const changed = state.diff.added + state.diff.changed + state.diff.removed;
    elements.statusMessage.textContent = changed ? "Canonical semantics changed; stable selection was preserved where possible" : state.snapshot.source.authority;
    elements.semanticDiff.innerHTML = `<span class="added">+${state.diff.added}</span><span class="changed">~${state.diff.changed}</span><span class="removed">−${state.diff.removed}</span>${state.diff.unresolved ? `<span class="removed">${state.diff.unresolved} unresolved</span>` : ""}`;
    elements.revision.textContent = state.snapshot.source.source_revision ?? "uncommitted source";
  }

  function selectNode(id, historyMode = "push") {
    const node = state.index.nodesById.get(id);
    if (!node) return showToast(`Deep link does not resolve: ${id}`);
    state.nodeId = id;
    state.edgeId = null;
    state.obligationId = null;
    state.moduleId = node.module_id;
    writeUrl(historyMode);
    render();
    openInspectorOnCompactScreens();
  }

  function selectModule(id, historyMode = "push") {
    if (!state.index.nodesById.has(id)) return;
    state.moduleId = id;
    state.nodeId = null;
    state.edgeId = null;
    state.obligationId = null;
    writeUrl(historyMode);
    render();
    closeCompactPanels();
  }

  function setView(view, historyMode = "push") {
    if (!model.VIEW_DEFINITIONS.some((item) => item.id === view)) return;
    state.view = view;
    state.edgeId = null;
    state.obligationId = null;
    writeUrl(historyMode);
    render();
    closeCompactPanels();
    elements.main.scrollTop = 0;
  }

  function viewForNode(node) {
    if (["machine", "state", "command", "observed-event", "effect-result", "transition-case", "event", "effect", "reply", "rejection"].includes(node.kind)) return "machines";
    if (["invariant", "evidence", "semantic-function"].includes(node.kind)) return "proofs";
    if (["trace-bundle", "trace-record"].includes(node.kind)) return "debug";
    if (model.BEHAVIOR_KINDS.has(node.kind) || ["public-behavior-binding", "dependency-behavior-binding"].includes(node.kind)) return "behaviors";
    return "system";
  }

  function openInspectorOnCompactScreens() {
    if (window.matchMedia("(max-width: 1120px)").matches) {
      elements.inspectorPane.classList.add("open");
      elements.scrim.classList.add("open");
    }
  }

  function closeCompactPanels() {
    elements.leftPane.classList.remove("open");
    elements.inspectorPane.classList.remove("open");
    elements.scrim.classList.remove("open");
  }

  function showToast(message) {
    elements.toast.textContent = message;
    elements.toast.classList.add("visible");
    window.clearTimeout(showToast.timer);
    showToast.timer = window.setTimeout(() => elements.toast.classList.remove("visible"), 1800);
  }

  document.addEventListener("click", async (event) => {
    const viewButton = event.target.closest("[data-view]");
    if (viewButton) return setView(viewButton.dataset.view);
    const nodeButton = event.target.closest("[data-node]");
    if (nodeButton) return selectNode(nodeButton.dataset.node);
    const moduleButton = event.target.closest("[data-module]");
    if (moduleButton) return selectModule(moduleButton.dataset.module);
    const edgeButton = event.target.closest("[data-edge]");
    if (edgeButton) {
      state.edgeId = edgeButton.dataset.edge;
      state.nodeId = null;
      state.obligationId = null;
      writeUrl(); render(); openInspectorOnCompactScreens(); return;
    }
    const obligationButton = event.target.closest("[data-obligation]");
    if (obligationButton) {
      const item = state.index.obligationsById.get(obligationButton.dataset.obligation);
      state.obligationId = item?.id ?? null;
      state.nodeId = null;
      state.edgeId = null;
      if (item) state.moduleId = item.module_id;
      writeUrl(); render(); openInspectorOnCompactScreens(); return;
    }
    const copyButton = event.target.closest("[data-copy]");
    if (copyButton) {
      await navigator.clipboard.writeText(copyButton.dataset.copy);
      showToast(`Copied ${copyButton.dataset.copy}`);
    }
  });

  document.addEventListener("dblclick", (event) => {
    const nodeButton = event.target.closest("[data-node]");
    if (!nodeButton) return;
    const node = state.index.nodesById.get(nodeButton.dataset.node);
    if (!node) return;
    state.view = viewForNode(node);
    selectNode(node.id);
  });

  document.addEventListener("change", (event) => {
    if (event.target.matches("[data-status-filter]")) {
      state.status = event.target.value;
      writeUrl("replace"); render();
    } else if (event.target.matches("[data-select-machine], [data-select-trace]")) {
      selectNode(event.target.value);
    }
  });

  elements.search.addEventListener("input", (event) => {
    state.query = event.target.value;
    render();
    window.clearTimeout(elements.search.historyTimer);
    elements.search.historyTimer = window.setTimeout(() => writeUrl("replace"), 180);
  });
  elements.refresh.addEventListener("click", () => loadSnapshot(true));
  elements.navToggle.addEventListener("click", () => { closeCompactPanels(); elements.leftPane.classList.add("open"); elements.scrim.classList.add("open"); });
  elements.inspectorClose.addEventListener("click", closeCompactPanels);
  elements.scrim.addEventListener("click", closeCompactPanels);
  window.addEventListener("popstate", () => {
    applyUrlState();
    normalizeSelection();
    render();
    if (state.nodeId || state.edgeId || state.obligationId) openInspectorOnCompactScreens();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "/" && !["INPUT", "SELECT", "TEXTAREA"].includes(document.activeElement?.tagName)) {
      event.preventDefault(); elements.search.focus();
    } else if (event.key === "Escape") {
      closeCompactPanels();
      if (document.activeElement === elements.search && state.query) { state.query = ""; elements.search.value = ""; writeUrl("replace"); render(); }
    } else if (["ArrowDown", "ArrowUp"].includes(event.key)) {
      const active = event.target.closest("button[data-node], button[data-module], button[data-obligation], button[data-view]");
      if (!active) return;
      const candidates = [...active.parentElement.querySelectorAll("button:not([disabled])")];
      const index = candidates.indexOf(active);
      const next = candidates[index + (event.key === "ArrowDown" ? 1 : -1)];
      if (next) { event.preventDefault(); next.focus(); }
    }
  });

  loadSnapshot(false);
}

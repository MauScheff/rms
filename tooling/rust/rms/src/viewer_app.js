"use strict";

(function bootstrap(global) {
  const VIEW_DEFINITIONS = [
    { id: "system", label: "Overview", description: "Purpose, ownership, composition, and semantic health." },
    { id: "proofs", label: "Laws", description: "Plain-language laws and their exact implementation and proof chains." },
    { id: "behaviors", label: "Behaviors", description: "Public promises and required capabilities through declared bindings." },
    { id: "machines", label: "Machines", description: "States, classified inputs, transition cases, outputs, and effects." },
    { id: "properties", label: "Properties", description: "Executable observations, assumptions, temporal expressions, verdicts, and relationships." },
    { id: "gaps", label: "Findings", description: "Missing, unresolved, recommended, satisfied, and inapplicable obligations." },
    { id: "debug", label: "Traces", description: "Execution-derived transition records and source provenance." },
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
    return String(a.label ?? a.title ?? a.id).localeCompare(String(b.label ?? b.title ?? b.id))
      || String(a.id).localeCompare(String(b.id));
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
    for (const values of obligationsByModule.values()) {
      values.sort((a, b) => STATUS_ORDER.indexOf(a.status) - STATUS_ORDER.indexOf(b.status) || compare(a, b));
    }
    return {
      snapshot, graph, nodesById, edgesById, obligationsById,
      outgoing, incoming, nodesByModule, obligationsByModule,
    };
  }

  function matchesQuery(value, query) {
    if (!query) return true;
    const refs = (value?.source_refs ?? []).map((source) => `${source.role} ${source.path}`).join(" ");
    const details = Object.entries(value?.details ?? {}).flat().join(" ");
    const lists = Object.values(value?.lists ?? {}).flat().join(" ");
    return text(`${value?.id} ${value?.kind} ${value?.label} ${value?.summary} ${value?.title} ${value?.detail} ${refs} ${details} ${lists}`)
      .includes(text(query));
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
      return STATUS_ORDER.indexOf(a.status) - STATUS_ORDER.indexOf(b.status) || compare(a, b);
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

  function closureObligations(index, node) {
    const items = index.obligationsByModule.get(node.module_id) ?? [];
    const quotedLabel = `\`${node.label}\``;
    const matchingKinds = node.kind === "invariant"
      ? new Set(["invariant-proof-chain"])
      : BEHAVIOR_KINDS.has(node.kind)
        ? new Set(["public-binding", "public-reachability", "public-proof-chain", "dependency-binding"])
        : null;
    const candidates = items.filter((item) => {
      if (matchingKinds && !matchingKinds.has(item.kind)) return false;
      return item.title.includes(quotedLabel)
        || item.detail.includes(quotedLabel)
        || item.id.includes(node.id.replaceAll(":", "-"));
    });
    const kindOrder = ["public-binding", "public-reachability", "dependency-binding", "public-proof-chain", "invariant-proof-chain"];
    return candidates.sort((a, b) => {
      return kindOrder.indexOf(a.kind) - kindOrder.indexOf(b.kind)
        || STATUS_ORDER.indexOf(a.status) - STATUS_ORDER.indexOf(b.status)
        || compare(a, b);
    });
  }

  function closureObligation(index, node) {
    return closureObligations(index, node)
      .sort((a, b) => STATUS_ORDER.indexOf(a.status) - STATUS_ORDER.indexOf(b.status))[0] ?? null;
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
    buildIndex, matchesQuery, nodes, obligations, statusCounts, moduleStatus, closureObligations, closureObligation,
    groupBy, semanticDiff, neighborhood, systemRelationships, traceRecords, parseUrl, urlFor,
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
    hydrated: false,
    pollTimer: null,
    diff: { added: 0, changed: 0, removed: 0, unresolved: 0 },
  };

  const elements = {
    systemName: document.querySelector("#system-name"),
    systemPurpose: document.querySelector("#system-purpose"),
    sourceState: document.querySelector("#source-state"),
    refresh: document.querySelector("#refresh"),
    tabs: document.querySelector("#tabs"),
    search: document.querySelector("#search"),
    moduleFilter: document.querySelector("#module-filter"),
    toolbarMeta: document.querySelector("#toolbar-meta"),
    workspace: document.querySelector("#workspace"),
    content: document.querySelector("#content"),
    detail: document.querySelector("#detail"),
    statusMessage: document.querySelector("#status-message"),
    semanticDiff: document.querySelector("#semantic-diff"),
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
    Object.assign(state, model.parseUrl(window.location));
    elements.search.value = state.query;
  }

  function writeUrl(mode = "push") {
    const method = mode === "replace" ? "replaceState" : "pushState";
    window.history[method]({}, "", model.urlFor(state, window.location));
  }

  function normalizeSelection() {
    if (state.nodeId && !state.index.nodesById.has(state.nodeId)) state.nodeId = null;
    if (state.edgeId && !state.index.edgesById.has(state.edgeId)) state.edgeId = null;
    if (state.obligationId && !state.index.obligationsById.has(state.obligationId)) state.obligationId = null;
    if (state.moduleId && !state.index.nodesById.has(state.moduleId)) state.moduleId = null;
  }

  async function loadSnapshot(manual = false) {
    if (state.loading) return;
    state.loading = true;
    if (manual) elements.statusMessage.textContent = "Refreshing canonical semantics…";
    try {
      const response = await fetch("/api/snapshot", { cache: "no-store" });
      if (!response.ok) throw new Error(`snapshot request failed (${response.status})`);
      const next = await response.json();
      state.diff = model.semanticDiff(state.snapshot, next);
      state.snapshot = next;
      state.index = model.buildIndex(next);
      if (!state.hydrated) {
        applyUrlState();
        state.hydrated = true;
      }
      normalizeSelection();
      render();
      configurePolling();
    } catch (error) {
      elements.content.innerHTML = `<div class="empty">${escapeHtml(error.message)}</div>`;
      elements.statusMessage.textContent = "Canonical projection unavailable";
      elements.sourceState.textContent = "Offline";
      elements.sourceState.classList.remove("live");
    } finally {
      state.loading = false;
    }
  }

  function configurePolling() {
    if (state.pollTimer || !state.snapshot?.source?.refresh_ms) return;
    state.pollTimer = window.setInterval(() => loadSnapshot(false), state.snapshot.source.refresh_ms);
  }

  function scopedNodes(kinds) {
    return model.nodes(state.index, {
      kinds,
      moduleId: state.moduleId,
      query: state.query,
    });
  }

  function scopedObligations(options = {}) {
    return model.obligations(state.index, {
      moduleId: state.moduleId,
      query: state.query,
      ...options,
    });
  }

  function viewCount(viewId) {
    if (viewId === "system") return model.nodes(state.index, { kinds: ["module"] }).length;
    if (viewId === "proofs") return model.nodes(state.index, { kinds: ["invariant"] }).length;
    if (viewId === "behaviors") return model.nodes(state.index, { kinds: [...model.BEHAVIOR_KINDS] }).length;
    if (viewId === "machines") return model.nodes(state.index, { kinds: ["machine"] }).length;
    if (viewId === "properties") return state.snapshot.properties?.length ?? 0;
    if (viewId === "gaps") return state.index.graph.obligations.filter((item) => !["satisfied", "not-applicable"].includes(item.status)).length;
    return model.nodes(state.index, { kinds: ["trace-record"] }).length;
  }

  function render() {
    if (!state.index) return;
    renderHeader();
    const renderers = {
      system: renderOverview,
      proofs: renderLaws,
      behaviors: renderBehaviors,
      machines: renderMachines,
      properties: renderProperties,
      gaps: renderFindings,
      debug: renderTraces,
    };
    elements.content.innerHTML = (renderers[state.view] ?? renderOverview)();
    renderDetail();
    renderFooter();
  }

  function renderHeader() {
    elements.systemName.textContent = state.snapshot.system.name;
    elements.systemPurpose.textContent = state.snapshot.system.purpose ?? "";
    const live = state.snapshot.source.refresh_ms > 0;
    elements.sourceState.textContent = live ? "Live" : "Snapshot";
    elements.sourceState.classList.toggle("live", live);
    elements.tabs.innerHTML = model.VIEW_DEFINITIONS.map((view) => `
      <button class="tab ${state.view === view.id ? "active" : ""}" type="button" data-view="${view.id}" title="${escapeHtml(view.description)}">
        ${escapeHtml(view.label)}<span class="tab-count">${viewCount(view.id)}</span>
      </button>
    `).join("");

    const modules = model.nodes(state.index, { kinds: ["module"] });
    elements.moduleFilter.innerHTML = `
      <option value="">All modules</option>
      ${modules.map((module) => `<option value="${escapeHtml(module.id)}" ${state.moduleId === module.id ? "selected" : ""}>${escapeHtml(module.label)}</option>`).join("")}
    `;
    const scoped = state.moduleId ? state.index.nodesByModule.get(state.moduleId)?.length ?? 0 : state.index.graph.nodes.length;
    elements.toolbarMeta.textContent = `${scoped} canonical objects`;
  }

  function viewHeader(title, description, tools = "") {
    return `
      <header class="view-header">
        <div><h2>${escapeHtml(title)}</h2><p>${escapeHtml(description)}</p></div>
        ${tools ? `<div class="view-tools">${tools}</div>` : ""}
      </header>
    `;
  }

  function section(title, count, body) {
    return `
      <section class="section">
        <div class="section-heading"><h3>${escapeHtml(title)}</h3><span>${escapeHtml(count)}</span></div>
        ${body}
      </section>
    `;
  }

  function statusLabel(status) {
    return status.replaceAll("-", " ");
  }

  function statusMarkup(status) {
    return `<span class="status ${escapeHtml(status)}">${escapeHtml(statusLabel(status))}</span>`;
  }

  function closureMarkup(obligation) {
    return obligation
      ? statusMarkup(obligation.status)
      : `<span class="closure-absent">No closure result</span>`;
  }

  function closureFor(node) {
    return model.closureObligation(state.index, node);
  }

  function closuresFor(node) {
    return model.closureObligations(state.index, node);
  }

  function moduleName(moduleId) {
    return state.index.nodesById.get(moduleId)?.label ?? moduleId;
  }

  function renderOverview() {
    const modules = scopedNodes(["module"]);
    const allObligations = scopedObligations();
    const counts = model.statusCounts(allObligations);
    const attention = allObligations.filter((item) => ["required-gap", "unresolved-link", "recommendation"].includes(item.status)).slice(0, 12);
    const relationships = model.systemRelationships(state.index).filter((edge) => {
      if (state.moduleId && edge.fromModule !== state.moduleId && edge.toModule !== state.moduleId) return false;
      if (!state.query) return true;
      return model.matchesQuery(edge, state.query)
        || model.matchesQuery(state.index.nodesById.get(edge.fromModule), state.query)
        || model.matchesQuery(state.index.nodesById.get(edge.toModule), state.query);
    });
    const invariantCount = scopedNodes(["invariant"]).length;
    return `
      ${viewHeader("Overview", "The system’s owned meaning, structure, and current semantic obligations.")}
      <div class="summary">
        <span><strong>${modules.length}</strong> modules</span>
        <span><strong>${invariantCount}</strong> laws</span>
        <span><strong>${counts.satisfied}</strong> satisfied</span>
        <span><strong>${counts["required-gap"] + counts["unresolved-link"]}</strong> blocking</span>
        <span><strong>${counts.recommendation}</strong> recommendations</span>
      </div>
      ${attention.length ? section("Needs attention", attention.length, `
        <div class="list">${attention.map(renderObligationRow).join("")}</div>
      `) : ""}
      ${section("Modules", modules.length, `
        <div class="list">${modules.map((module) => {
          const status = model.moduleStatus(state.index, module.id);
          return `<button class="row ${state.nodeId === module.id ? "active" : ""}" type="button" data-node="${escapeHtml(module.id)}">
            <span class="row-kind">${statusMarkup(status)}</span>
            <span class="row-main"><strong>${escapeHtml(module.label)}</strong><p>${escapeHtml(module.summary)}</p></span>
            <span class="row-side">${escapeHtml(module.details?.shape || module.details?.kind || "module")}</span>
          </button>`;
        }).join("") || `<div class="empty">No modules match this scope.</div>`}</div>
      `)}
      ${section("Declared relationships", relationships.length, `
        <div class="list">${relationships.map((edge) => {
          const from = state.index.nodesById.get(edge.fromModule);
          const to = state.index.nodesById.get(edge.toModule);
          return `<button class="row relation-row ${state.edgeId === edge.id ? "active" : ""}" type="button" data-edge="${escapeHtml(edge.id)}">
            <span class="row-main"><strong>${escapeHtml(from?.label ?? edge.fromModule)}</strong></span>
            <span class="row-kind">${escapeHtml(edge.label)}</span>
            <span class="row-main"><strong>${escapeHtml(to?.label ?? edge.toModule)}</strong></span>
          </button>`;
        }).join("") || `<div class="empty">No cross-module relationships match this scope.</div>`}</div>
      `)}
    `;
  }

  function renderLaws() {
    const laws = scopedNodes(["invariant"]).sort((left, right) => {
      const leftStatus = closureFor(left)?.status;
      const rightStatus = closureFor(right)?.status;
      const leftOrder = leftStatus ? model.STATUS_ORDER.indexOf(leftStatus) : 3.5;
      const rightOrder = rightStatus ? model.STATUS_ORDER.indexOf(rightStatus) : 3.5;
      return leftOrder - rightOrder || left.summary.localeCompare(right.summary) || left.id.localeCompare(right.id);
    });
    return `
      ${viewHeader("Laws", "Plain-language truths first. Select a law to inspect its authority, implementation owner, proof, and source.")}
      <div class="list">${laws.map((law) => {
        const obligation = closureFor(law);
        return `<button class="row law-row ${state.nodeId === law.id ? "active" : ""}" type="button" data-node="${escapeHtml(law.id)}">
          <span class="row-kind">${closureMarkup(obligation)}</span>
          <span class="row-main"><strong>${escapeHtml(law.summary)}</strong><p>${escapeHtml(law.label)} · ${escapeHtml(moduleName(law.module_id))}</p></span>
          <span class="row-side">${escapeHtml(law.details?.authority || "authority not declared")}</span>
        </button>`;
      }).join("") || `<div class="empty">No laws match the current scope.</div>`}</div>
    `;
  }

  function renderBehaviors() {
    const behaviors = scopedNodes([...model.BEHAVIOR_KINDS]);
    return `
      ${viewHeader("Behaviors", "Public promises and required capabilities, with their declared semantic closure.")}
      <div class="list">${behaviors.map((behavior) => {
        const obligation = closureFor(behavior);
        return `<button class="row ${state.nodeId === behavior.id ? "active" : ""}" type="button" data-node="${escapeHtml(behavior.id)}">
          <span class="row-kind">${escapeHtml(behavior.kind.replaceAll("-", " "))}</span>
          <span class="row-main"><strong>${escapeHtml(behavior.label)}</strong><p>${escapeHtml(behavior.summary)}</p></span>
          <span class="row-side">${closureMarkup(obligation)}<br>${escapeHtml(moduleName(behavior.module_id))}</span>
        </button>`;
      }).join("") || `<div class="empty">No behaviors match the current scope.</div>`}</div>
    `;
  }

  function activeMachine(machines) {
    const selected = selectedNode();
    if (selected?.kind === "machine") return selected;
    if (selected && machines.some((machine) => machine.module_id === selected.module_id)) {
      return machines.find((machine) => machine.module_id === selected.module_id) ?? machines[0];
    }
    return machines[0] ?? null;
  }

  function renderMachines() {
    const machines = scopedNodes(["machine"]);
    const machine = activeMachine(machines);
    if (!machine) {
      return `${viewHeader("Machines", "Lifecycle state, accepted inputs, transitions, outputs, and effects.")}<div class="empty">No implemented machine matches this scope.</div>`;
    }
    const moduleNodes = state.index.nodesByModule.get(machine.module_id) ?? [];
    const states = moduleNodes.filter((node) => node.kind === "state").sort((a, b) => a.label.localeCompare(b.label));
    const transitions = moduleNodes
      .filter((node) => node.kind === "transition-case" && model.matchesQuery(node, state.query))
      .sort((a, b) => a.label.localeCompare(b.label));
    return `
      ${viewHeader("Machines", "Lifecycle state, accepted inputs, transitions, outputs, and effects.", machines.length > 1 ? `
        <select class="select" id="machine-select" aria-label="Select machine">
          ${machines.map((item) => `<option value="${escapeHtml(item.id)}" ${item.id === machine.id ? "selected" : ""}>${escapeHtml(item.label)}</option>`).join("")}
        </select>
      ` : "")}
      <div class="summary">
        <span><strong>${escapeHtml(machine.label)}</strong></span>
        <span>${escapeHtml(moduleName(machine.module_id))}</span>
        <span><strong>${transitions.length}</strong> transition cases</span>
      </div>
      ${section("States", states.length, `<div class="states">${states.map((item) => `
        <button class="state ${item.label === machine.details?.initial_state ? "initial" : ""}" type="button" data-node="${escapeHtml(item.id)}">${escapeHtml(item.label)}</button>
      `).join("") || `<span class="state">Unit state</span>`}</div>`)}
      ${section("Transition cases", transitions.length, `
        <div class="list">${transitions.map((transition) => `
          <button class="row transition-row ${state.nodeId === transition.id ? "active" : ""}" type="button" data-node="${escapeHtml(transition.id)}">
            <span class="row-main"><strong>${escapeHtml(transition.label)}</strong></span>
            <span class="row-kind">${escapeHtml(transition.details?.from || "?")}</span>
            <span class="row-main"><strong>${escapeHtml(transition.details?.on || "?")}</strong></span>
            <span class="row-side">${escapeHtml(transition.details?.to || "?")}</span>
          </button>
        `).join("") || `<div class="empty">No transition cases match.</div>`}</div>
      `)}
    `;
  }

  function renderFindings() {
    const all = scopedObligations();
    const counts = model.statusCounts(all);
    const items = scopedObligations({ status: state.status });
    const tools = `
      <select class="select" id="status-filter" aria-label="Filter findings by status">
        <option value="all">All statuses</option>
        ${model.STATUS_ORDER.map((status) => `<option value="${status}" ${state.status === status ? "selected" : ""}>${escapeHtml(statusLabel(status))} (${counts[status] ?? 0})</option>`).join("")}
      </select>
    `;
    return `
      ${viewHeader("Findings", "Missing, unresolved, recommended, satisfied, and inapplicable are kept distinct.", tools)}
      <div class="list">${items.map(renderObligationRow).join("") || `<div class="empty">No findings match this filter.</div>`}</div>
    `;
  }

  function renderProperties() {
    const properties = (state.snapshot.properties ?? []).filter((property) => {
      if (state.moduleId && !state.moduleId.endsWith(`:${property.module_id}`)) return false;
      if (!state.query) return true;
      return text(JSON.stringify(property)).includes(text(state.query));
    });
    const analyses = (state.snapshot.property_analyses ?? []).filter((analysis) => {
      if (!state.query) return true;
      return text(JSON.stringify(analysis)).includes(text(state.query));
    });
    return `
      ${viewHeader("Executable properties", "Typed observations and assumptions feed one evaluator used by traces, search, replay, and monitors.")}
      <div class="summary">
        <span><strong>${properties.length}</strong> declarations</span>
        <span><strong>${analyses.length}</strong> recorded analyses</span>
      </div>
      ${section("Declarations", properties.length, `<div class="list">${properties.map((property) => `
        <article class="row">
          <span class="row-kind">${statusMarkup(property.status === "declared" ? "satisfied" : "required-gap")}</span>
          <span class="row-main">
            <strong>${escapeHtml(property.id)}</strong>
            <p>${escapeHtml(property.module_id)} · ${escapeHtml(property.scope)} · ${property.observations.length} observations · ${property.assumptions.length} assumptions</p>
            <pre>${escapeHtml(JSON.stringify(property.expression, null, 2))}</pre>
          </span>
        </article>
      `).join("") || `<div class="empty">No executable properties match this scope.</div>`}</div>`)}
      ${section("Analysis history", analyses.length, `<div class="list">${analyses.map((analysis) => `
        <article class="row">
          <span class="row-kind">${escapeHtml(analysis.result)}</span>
          <span class="row-main"><strong>${escapeHtml(analysis.operation)}</strong><p>${escapeHtml(analysis.path)}</p></span>
          <span class="row-side">${analysis.evaluations.length} verdicts<br>${analysis.relationships.length} relationships</span>
        </article>
      `).join("") || `<div class="empty">No rms/property-analysis/v0.1 artifacts are recorded.</div>`}</div>`)}
    `;
  }

  function renderObligationRow(item) {
    return `<button class="row ${state.obligationId === item.id ? "active" : ""}" type="button" data-obligation="${escapeHtml(item.id)}">
      <span class="row-kind">${statusMarkup(item.status)}</span>
      <span class="row-main"><strong>${escapeHtml(item.title)}</strong><p>${escapeHtml(item.detail)}</p></span>
      <span class="row-side">${escapeHtml(moduleName(item.module_id))}<br>${escapeHtml(item.kind)}</span>
    </button>`;
  }

  function activeTraceBundle(bundles) {
    const selected = selectedNode();
    if (selected?.kind === "trace-bundle") return selected;
    if (selected?.kind === "trace-record") {
      const edge = (state.index.incoming.get(selected.id) ?? []).find((candidate) => candidate.kind === "contains");
      if (edge) return state.index.nodesById.get(edge.from) ?? bundles[0] ?? null;
    }
    return bundles[0] ?? null;
  }

  function renderTraces() {
    const bundles = scopedNodes(["trace-bundle"]);
    const bundle = activeTraceBundle(bundles);
    if (!bundle) {
      return `${viewHeader("Traces", "Execution-derived transition records. No synthetic history is shown.")}<div class="empty">No trace bundle matches this scope.</div>`;
    }
    const records = model.traceRecords(state.index, bundle.id).filter((record) => model.matchesQuery(record, state.query));
    return `
      ${viewHeader("Traces", "Execution-derived transition records. No synthetic history is shown.", bundles.length > 1 ? `
        <select class="select" id="trace-select" aria-label="Select trace bundle">
          ${bundles.map((item) => `<option value="${escapeHtml(item.id)}" ${item.id === bundle.id ? "selected" : ""}>${escapeHtml(moduleName(item.module_id))} · ${escapeHtml(item.label)}</option>`).join("")}
        </select>
      ` : "")}
      <div class="summary"><span><strong>${escapeHtml(bundle.label)}</strong></span><span>${escapeHtml(bundle.summary)}</span></div>
      <div class="list">${records.map((record) => `
        <button class="row trace-row ${state.nodeId === record.id ? "active" : ""}" type="button" data-node="${escapeHtml(record.id)}">
          <span class="row-kind">${record.details?.rejection ? statusMarkup("required-gap") : statusMarkup("satisfied")}</span>
          <span class="row-main"><strong>${escapeHtml(record.label)}</strong><p>${escapeHtml(record.details?.input || record.summary)}</p></span>
          <span class="row-side">${escapeHtml(record.details?.state_before || "?")} → ${escapeHtml(record.details?.state_after || "?")}</span>
        </button>
      `).join("") || `<div class="empty">No execution-derived records match.</div>`}</div>
    `;
  }

  function renderDetail() {
    const node = selectedNode();
    const edge = selectedEdge();
    const obligation = selectedObligation();
    elements.workspace.classList.toggle("has-detail", Boolean(node || edge || obligation));
    if (node) elements.detail.innerHTML = nodeDetail(node);
    else if (edge) elements.detail.innerHTML = edgeDetail(edge);
    else if (obligation) elements.detail.innerHTML = obligationDetail(obligation);
    else elements.detail.innerHTML = "";
  }

  function detailHeader(title) {
    return `<div class="detail-head"><h2>${escapeHtml(title)}</h2><button class="close-button" type="button" data-close-detail>Close</button></div>`;
  }

  function detailsMarkup(details, lists = {}) {
    const rows = [
      ...Object.entries(details ?? {}).filter(([, value]) => value),
      ...Object.entries(lists ?? {}).filter(([, value]) => value?.length).map(([key, value]) => [key, value.join(", ")]),
    ];
    return rows.length ? `<dl class="detail-list">${rows.map(([key, value]) => `
      <div><dt>${escapeHtml(key.replaceAll("_", " "))}</dt><dd>${escapeHtml(value)}</dd></div>
    `).join("")}</dl>` : `<p>No additional canonical fields.</p>`;
  }

  function sourcesMarkup(refs) {
    return (refs ?? []).length ? refs.map((source) => `
      <button class="source" type="button" data-copy="${escapeHtml(source.path)}">${escapeHtml(source.role)} · ${escapeHtml(source.path)}</button>
    `).join("") : `<p>No source reference declared.</p>`;
  }

  function nodeDetail(node) {
    const module = moduleName(node.module_id);
    const closures = closuresFor(node);
    const connections = [
      ...(state.index.outgoing.get(node.id) ?? []).map((edge) => ({ edge, neighbor: state.index.nodesById.get(edge.to), direction: "→" })),
      ...(state.index.incoming.get(node.id) ?? []).map((edge) => ({ edge, neighbor: state.index.nodesById.get(edge.from), direction: "←" })),
    ].filter((item) => item.neighbor).sort((a, b) => a.edge.id.localeCompare(b.edge.id));
    return `
      ${detailHeader(node.kind === "invariant" ? node.summary : node.label)}
      <p>${escapeHtml(node.kind === "invariant" ? node.label : node.summary)}</p>
      <p>${escapeHtml(node.kind.replaceAll("-", " "))} · ${escapeHtml(module)}</p>
      ${closures.length ? `<h3>Closure</h3><div class="closure-steps">${closures.map((closure) => `
        <div class="closure-step">
          <div>${statusMarkup(closure.status)} <span>${escapeHtml(closure.kind.replaceAll("-", " "))}</span></div>
          <p>${escapeHtml(closure.detail)}</p>
        </div>
      `).join("")}</div>` : ""}
      <h3>Canonical fields</h3>
      ${detailsMarkup(node.details, node.lists)}
      <h3>Declared connections</h3>
      ${connections.map(({ edge, neighbor, direction }) => `
        <button class="connection" type="button" data-node="${escapeHtml(neighbor.id)}">
          <strong>${direction} ${escapeHtml(edge.label)}</strong>${escapeHtml(neighbor.label)} · ${escapeHtml(neighbor.kind)}
        </button>
      `).join("") || `<p>No declared graph neighbors.</p>`}
      <h3>Source</h3>
      ${sourcesMarkup(node.source_refs)}
    `;
  }

  function edgeDetail(edge) {
    const from = state.index.nodesById.get(edge.from);
    const to = state.index.nodesById.get(edge.to);
    return `
      ${detailHeader(edge.label)}
      <p>${escapeHtml(edge.kind.replaceAll("-", " "))}</p>
      <h3>Endpoints</h3>
      <button class="connection" type="button" data-node="${escapeHtml(from?.id ?? edge.from)}"><strong>From</strong>${escapeHtml(from?.label ?? edge.from)}</button>
      <button class="connection" type="button" data-node="${escapeHtml(to?.id ?? edge.to)}"><strong>To</strong>${escapeHtml(to?.label ?? edge.to)}</button>
      <h3>Source</h3>
      ${sourcesMarkup(edge.source_refs)}
    `;
  }

  function obligationDetail(item) {
    return `
      ${detailHeader(item.title)}
      ${statusMarkup(item.status)}
      <p>${escapeHtml(item.detail)}</p>
      <h3>Meaning</h3>
      <p>${item.status === "not-applicable"
        ? "This obligation does not belong to the declared module shape."
        : item.status === "recommendation"
          ? "This would strengthen reliability but is not currently production-blocking."
          : item.status === "satisfied"
            ? "The applicable semantic chain is closed."
            : "An applicable semantic step is absent or does not resolve."}</p>
      <h3>Source</h3>
      ${sourcesMarkup(item.source_refs)}
    `;
  }

  function renderFooter() {
    const changed = state.diff.added + state.diff.changed + state.diff.removed;
    elements.statusMessage.textContent = changed
      ? "Canonical semantics changed; stable selection was preserved where possible"
      : state.snapshot.source.authority;
    elements.semanticDiff.innerHTML = `
      <span class="added">+${state.diff.added}</span>
      <span class="changed">~${state.diff.changed}</span>
      <span class="removed">−${state.diff.removed}</span>
      ${state.diff.unresolved ? `<span class="removed">${state.diff.unresolved} unresolved</span>` : ""}
    `;
    elements.revision.textContent = state.snapshot.source.source_revision ?? "uncommitted source";
  }

  function selectNode(id, mode = "push") {
    const node = state.index.nodesById.get(id);
    if (!node) return showToast(`Deep link does not resolve: ${id}`);
    state.nodeId = id;
    state.edgeId = null;
    state.obligationId = null;
    writeUrl(mode);
    render();
  }

  function selectEdge(id) {
    if (!state.index.edgesById.has(id)) return;
    state.edgeId = id;
    state.nodeId = null;
    state.obligationId = null;
    writeUrl();
    render();
  }

  function selectObligation(id) {
    if (!state.index.obligationsById.has(id)) return;
    state.obligationId = id;
    state.nodeId = null;
    state.edgeId = null;
    writeUrl();
    render();
  }

  function setView(view) {
    if (!model.VIEW_DEFINITIONS.some((item) => item.id === view)) return;
    state.view = view;
    state.nodeId = null;
    state.edgeId = null;
    state.obligationId = null;
    writeUrl();
    render();
    window.scrollTo({ top: 0, behavior: "smooth" });
  }

  function closeDetail() {
    state.nodeId = null;
    state.edgeId = null;
    state.obligationId = null;
    writeUrl("replace");
    render();
  }

  function showToast(message) {
    elements.toast.textContent = message;
    elements.toast.classList.add("visible");
    window.clearTimeout(showToast.timer);
    showToast.timer = window.setTimeout(() => elements.toast.classList.remove("visible"), 1600);
  }

  document.addEventListener("click", async (event) => {
    const view = event.target.closest("[data-view]");
    if (view) return setView(view.dataset.view);
    const node = event.target.closest("[data-node]");
    if (node) return selectNode(node.dataset.node);
    const edge = event.target.closest("[data-edge]");
    if (edge) return selectEdge(edge.dataset.edge);
    const obligation = event.target.closest("[data-obligation]");
    if (obligation) return selectObligation(obligation.dataset.obligation);
    const close = event.target.closest("[data-close-detail]");
    if (close) return closeDetail();
    const copy = event.target.closest("[data-copy]");
    if (copy) {
      await navigator.clipboard.writeText(copy.dataset.copy);
      showToast(`Copied ${copy.dataset.copy}`);
    }
  });

  document.addEventListener("change", (event) => {
    if (event.target === elements.moduleFilter) {
      state.moduleId = event.target.value || null;
      state.nodeId = null;
      state.edgeId = null;
      state.obligationId = null;
      writeUrl("replace");
      render();
    } else if (event.target.matches("#status-filter")) {
      state.status = event.target.value;
      writeUrl("replace");
      render();
    } else if (event.target.matches("#machine-select, #trace-select")) {
      selectNode(event.target.value, "replace");
    }
  });

  elements.search.addEventListener("input", (event) => {
    state.query = event.target.value;
    render();
    window.clearTimeout(elements.search.historyTimer);
    elements.search.historyTimer = window.setTimeout(() => writeUrl("replace"), 180);
  });

  elements.refresh.addEventListener("click", () => loadSnapshot(true));
  window.addEventListener("popstate", () => {
    applyUrlState();
    normalizeSelection();
    render();
  });
  document.addEventListener("keydown", (event) => {
    if (event.key === "/" && !["INPUT", "SELECT", "TEXTAREA"].includes(document.activeElement?.tagName)) {
      event.preventDefault();
      elements.search.focus();
    } else if (event.key === "Escape") {
      if (state.nodeId || state.edgeId || state.obligationId) closeDetail();
      else if (state.query) {
        state.query = "";
        elements.search.value = "";
        writeUrl("replace");
        render();
      }
    }
  });

  loadSnapshot(false);
}

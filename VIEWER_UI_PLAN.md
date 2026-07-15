# RMS Semantic Explorer UI Plan

## Purpose

Build a read-only web explorer that lets a developer, builder, or reviewer understand an RMS system without reading every canonical YAML file.

The viewer presents RMS semantics. It does not own or edit them.

```text
canonical RMS artifacts -> semantic system graph -> focused read-only views
```

The existing `rms/semantic-system-graph/v0.1` backend is the authority for nodes, edges, obligations, status, and source provenance. UI work must not add heuristic relationships, reinterpret missing data, or introduce a second architecture model.

## Scope

This plan covers:

- information architecture;
- graph exploration;
- behavior-path tracing;
- machine inspection;
- proof and gap inspection;
- source provenance;
- live refresh;
- responsive and accessible interaction;
- browser verification and performance budgets.

This plan excludes:

- editing manifests or source files;
- invoking `rms spec apply`, `rms machine apply`, or `rms surface apply` from the browser;
- a visual programming language;
- inferred relationships based on names or source-code similarity;
- a required RMS runtime;
- a blind new-project trial.

## Product Questions

The explorer must answer five questions directly:

1. **Understand:** What modules exist, what do they own, and how do they compose?
2. **Trace:** How does one public behavior travel from contract to implementation, machine cases, effects, and proof?
3. **Debug:** Which state or transition first violated a declared promise?
4. **Change:** Which semantic objects and module boundaries are likely affected by a requested change?
5. **Verify:** Which promises are proved, unresolved, recommended, or not applicable?

These are views over one graph, not independent data models.

## Semantic Status

The UI must preserve the backend status vocabulary exactly:

| Status | Meaning | Presentation |
| --- | --- | --- |
| `satisfied` | An applicable obligation is closed | Quiet success mark |
| `required-gap` | Required semantic structure or proof is absent | High-priority gap |
| `unresolved-link` | Both ends are declared but do not resolve | Broken-link finding |
| `recommendation` | Useful strengthening, not a production requirement | Advisory finding |
| `not-applicable` | The module shape does not own this obligation | Muted explicit state |

The interface must never collapse `not-applicable` into missing, hide unresolved links, or count recommendations as failures.

## Information Architecture

### Application Shell

Use a quiet, work-focused shell rather than a landing page:

- top toolbar: repository identity, source revision, refresh state, search, view switcher;
- left navigation: modules, public behaviors, required capabilities, machines, effects, proofs, gaps;
- main canvas: the selected semantic view;
- right inspector: selected object, relationships, obligations, and source references;
- optional bottom timeline: transition records and first-bad-transition context.

Desktop uses stable resizable panes. Tablet collapses the inspector into a slide-over panel. Mobile uses one primary pane with explicit back navigation and a bottom sheet for details. No nested cards or oversized display type.

### Primary Views

#### System Map

Default view for system-level understanding:

- modules grouped by bounded context and composition ownership;
- composite containment and public export edges;
- required capability edges to module or external providers;
- compact status summary per module;
- shape and binding labels;
- filters for context, shape, profile, status, and implementation binding.

The system map should optimize for scanning and dependency direction. It must not render every inner machine case at once.

#### Behavior Path

Focused view for one public command, query, or capability:

```text
public behavior
-> contract
-> public behavior binding
-> semantic function
-> machine input/cases/output
-> effects or dependency calls
-> evidence and trace coverage
```

Required-capability paths continue through:

```text
required capability
-> dependency behavior binding
-> exact local consumer
-> provider module/contract or external boundary
```

Every step displays its canonical source reference. Missing and unresolved steps remain in place as explicit nodes.

#### Machine View

Focused view for one domain-named machine:

- states and initial/terminal classification;
- commands, observed events, and effect results as distinct input categories;
- transition cases with exact from/on/to semantics;
- emitted events, commands, effects, replies, and rejections;
- effect protocols and executor ownership;
- trace coverage per case;
- unreachable or unrepresented cases.

Use a compact directed graph for state transitions and a synchronized transition table for precise comparison. Selection in either representation selects the same semantic case.

#### Proof View

Promise-first view:

- laws, contracts, properties, and temporal claims;
- authority or semantic-function owner;
- executable realization or evidence file;
- trace producer and represented cases;
- source revision and command provenance;
- status and exact missing link.

Evidence prose is secondary to the executable chain. A file existing is not visually presented as proof unless the backend marks the chain satisfied.

#### Gap Triage

Operational list ordered by severity and semantic reach:

- required gaps;
- unresolved links;
- recommendations;
- optionally visible not-applicable items.

Each finding provides:

- owning module;
- affected public behavior or promise;
- missing/broken graph step;
- canonical source locations;
- relevant RMS command category, without creating or applying changes.

#### Debug Timeline

When trace records exist:

- show command/event/effect-result inputs in order;
- preserve correlation, causation, sequence, and source provenance;
- show state before and after;
- show emitted outputs;
- highlight the first record that diverges from the declared transition or violated promise;
- link directly to the owning machine case and source file.

Absence of executable trace evidence is displayed as a proof gap, not replaced with a synthetic timeline.

## Interaction Model

### Navigation

- Every node has a stable, copyable deep link based on its semantic graph ID.
- Browser back/forward restores view, selection, filters, and inspector state.
- Search covers canonical labels, IDs, module names, contract names, machine cases, and source paths.
- Keyboard navigation moves through lists and connected graph nodes without requiring pointer input.
- Breadcrumbs show system -> module -> behavior/machine/promise -> selected case.

### Graph Interaction

- Single click selects.
- Double click or Enter focuses a neighborhood.
- Expand actions reveal one semantic layer at a time.
- “Show path” isolates the shortest declared path between two selected semantic objects.
- “Why is this connected?” lists the exact canonical edge and source references.
- “Why is this missing?” shows the obligation, applicability reason, and absent graph step.

Do not use unconstrained force layouts as the only representation. Preserve deterministic ordering and provide tables for exact comparison.

### Source Provenance

The inspector exposes source references as first-class data:

- artifact role;
- repository-relative path;
- semantic item ID;
- source revision;
- optional line or symbol when supplied by the backend.

Source links may open local files through the host environment when supported. The viewer itself remains read-only.

### Live Refresh

`--watch` refreshes the graph while preserving the current selection when stable IDs survive.

Refresh behavior:

- show added, changed, removed, and newly unresolved objects;
- never silently move focus to another semantic object;
- explain when the selected object was removed;
- debounce filesystem bursts;
- retain the previous valid snapshot when a refresh cannot parse canonical artifacts;
- display parse/validation failure separately from system semantic gaps.

## Visual System

Use a restrained operational palette with multiple semantic families:

- neutral white/charcoal foundation;
- blue for selected structure and navigation;
- green for satisfied obligations;
- red for required gaps and broken links;
- amber for recommendations;
- gray for not-applicable stages;
- violet only as a limited marker for proof/evidence objects.

Constraints:

- no gradients, decorative orbs, hero sections, or marketing composition;
- no page-section cards or cards nested inside cards;
- repeated semantic items may use compact rows or bounded panels with radius no greater than 8px;
- use Lucide icons for familiar controls;
- icon-only controls require tooltips and accessible names;
- text never scales from viewport width;
- graph labels use stable wrapping or truncation with full text in the inspector;
- status is conveyed by icon and text, not color alone;
- transitions, filters, and selection changes must not resize the shell.

## Data Contract

The initial UI consumes the existing snapshot and graph:

```text
GET /api/snapshot
  system metadata
  source revision
  modules and existing view summaries
  graph.nodes[]
  graph.edges[]
  graph.obligations[]
```

Required graph invariants:

- stable unique IDs;
- every edge endpoint resolves or is represented as an unresolved obligation;
- every object has one owning module;
- every displayed fact has at least one source reference;
- obligation status uses the closed vocabulary above;
- no UI-only semantic edges are persisted or reported as canonical.

If payload size becomes material, add read-only indexed endpoints without changing semantics:

```text
GET /api/graph/summary
GET /api/modules/:id
GET /api/nodes/:id
GET /api/nodes/:id/neighborhood?depth=1
GET /api/behaviors/:id/path
GET /api/machines/:id
GET /api/proofs/:id
GET /api/gaps
```

The monolithic snapshot remains supported until indexed endpoints are proven necessary by measured payload or render cost.

## Implementation Phases

### Phase 1: View Model

- Add deterministic client indexes by node kind, module, status, and edge direction.
- Add selectors for system map, behavior path, machine, proof, and gap views.
- Add deep-link serialization and restoration.
- Keep selectors pure and unit tested.

Done when every primary view can be produced from a fixture graph without DOM code.

### Phase 2: Application Shell

- Build stable toolbar, navigation, canvas, inspector, and optional timeline regions.
- Add responsive pane behavior.
- Add search, filters, breadcrumbs, and keyboard focus management.
- Preserve current loopback/read-only server behavior.

Done when desktop, tablet, and mobile shells render without overlap or horizontal overflow.

### Phase 3: System And Behavior Views

- Implement the system map with deterministic grouping and edge routing.
- Implement behavior-path tracing with explicit missing steps.
- Implement source-provenance inspector.
- Add stable deep links.

Done when a user can answer who owns a public behavior, which contract it promises, which machine cases implement it, and which proof covers it.

### Phase 4: Machine And Proof Views

- Implement synchronized state graph and transition table.
- Implement effect-protocol and executor inspection.
- Implement promise/proof chains and trace coverage.
- Implement gap triage ordered by semantic reach.

Done when each stateful case, effect result, rejection, and unproved promise is directly inspectable.

### Phase 5: Debug And Refresh

- Implement replay timeline and first-bad-transition focus.
- Add watch-mode semantic diff.
- Preserve selection across refresh.
- Distinguish parse failures from semantic findings.

Done when a changed or failing trace can be followed from outside input to its first invalid transition or unresolved handoff.

### Phase 6: Hardening

- Add accessibility audit and keyboard-only verification.
- Add large-graph virtualization and measured rendering thresholds.
- Add error, empty, loading, and disconnected states.
- Add browser screenshots and pixel/overflow checks.
- Update documentation and contract evidence from executed results.

Done when all acceptance criteria below pass.

## Verification

### Unit Tests

- graph selectors preserve status and source provenance;
- behavior paths never invent edges;
- missing links remain visible;
- shape-inapplicable obligations remain `not-applicable`;
- deterministic ordering produces stable snapshots;
- URL state round-trips selection and filters;
- semantic diff classifies added, changed, removed, and unresolved objects.

### Browser Tests

Use Playwright against a real `rms view --port 0 --no-open` process.

Required scenarios:

- open system map and select a module;
- follow a public behavior from contract to machine and proof;
- inspect a required capability through its consumer and provider;
- inspect a state transition and its replay evidence;
- distinguish required gap, recommendation, and not-applicable status;
- search and deep-link to an exact semantic object;
- refresh after a canonical artifact change while preserving selection;
- navigate entirely by keyboard;
- reject unsupported HTTP methods and routes.

Capture and inspect screenshots at:

- 1440 x 900 desktop;
- 1024 x 768 tablet;
- 390 x 844 mobile.

Verify no blank canvas, overlapping controls, clipped status text, horizontal overflow, or incoherent graph framing.

### Performance Budgets

Initial targets, measured on a local release build:

- first useful system map under 1 second for 1,000 graph nodes;
- selection-to-inspector update under 100 ms;
- search result update under 100 ms for 10,000 indexed objects;
- no main-thread task above 100 ms during ordinary navigation;
- bounded memory through list and graph virtualization above measured thresholds.

Revise budgets only from recorded measurements.

## Acceptance Criteria

- A person unfamiliar with the source can identify module ownership and public behavior paths from the viewer alone.
- A public behavior path visibly closes contract, semantic owner, machine cases, and proof.
- A required capability visibly closes exact consumer and provider/external resolution.
- Pure modules and composites show correct non-applicable stages rather than false gaps.
- Missing and unresolved links are never hidden or heuristically repaired.
- Every displayed semantic fact links back to canonical provenance.
- Stateful behavior can be inspected as states, classified inputs, exact transition cases, outputs, and replay records.
- The viewer remains read-only and cannot become semantic authority.
- Desktop, tablet, and mobile browser checks pass with no overlap or overflow.
- Accessibility, keyboard navigation, and performance budgets pass.

## Release Gate

Before claiming the redesigned viewer complete:

```bash
cargo fmt --all --check
cargo test --workspace --locked
rms validate --root .
rms compose --root .
rms gate --root .
git commit
rms audit --root . --strict
```

Update `tooling/rust/rms/verification/contracts/view_rms_system.md` with the exact executed browser commands, viewport results, observed graph fixture, and committed source revision.

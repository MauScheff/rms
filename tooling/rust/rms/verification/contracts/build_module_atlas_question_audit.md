# Module Atlas Journey Probe Audit

## Purpose

This audit uses user questions as design probes. They are not a proposed literal Q&A interface.

The goal is to redesign `rms atlas` from a graph-first visualization into a guided semantic inspection tool. The interface should help a human understand a module through orientation, focus, trace, source evidence, and next action. The 3D map remains useful as spatial context, but the user should not need to decode raw graph adjacency.

## Scope

Target artifact:

- `dist/payments-atlas/index.html`
- `dist/payments-atlas/atlas.json`
- source module: `examples/commerce/payments.module.yaml`
- generated source revision: `git:be8eca7ef485`

Captured evidence:

- `dist/payments-atlas/audit/01-overview.png`
- `dist/payments-atlas/audit/02-public-surface.png`
- `dist/payments-atlas/audit/03-capture-payment.png`
- `dist/payments-atlas/audit/04-effect.png`
- `dist/payments-atlas/audit/05-proof.png`

Observed graph shape:

- 33 nodes
- 32 edges
- 8 layers: overview, ownership, public-surface, dependencies, effects, constraints, lifecycle, verification
- 1 deterministic tour: `human-overview`

## Design Judgment

The current atlas is visually legible as a semantic map, but it is still graph-first. It helps the user see that semantic objects exist. It does not yet help the user move through a maintainer journey.

The core issue is not 3D navigation. It is interaction composition. A maintainer should not need to click through raw adjacency to infer that changing `capture-payment` touches a public contract, authorization invariant, payment provider effect, state lifecycle, compatibility policy, and verification evidence.

The next atlas should be journey-led:

1. The user enters through a semantic object or task mode, not a written question.
2. The interface focuses the map on the relevant region.
3. The map highlights the supporting trace.
4. The side panel explains why each highlighted element matters.
5. The UI offers source refs, gaps, and next actions.

## Journey Probes

### 1. First-Minute Orientation

Probe: "What is this module for, and why does it exist?"

Current behavior:

- The header states `payments`.
- The purpose says "Own payment authorization, capture, refund, and provider reconciliation."
- The center node repeats the same meaning.

Gap:

- The surface does not turn purpose into a working mental model.
- The first inspector state is a list of adjacent profiles and concepts.
- The user has no clear path from orientation into ownership, public surface, risk, or proof.

Interaction outcome:

- Lead with the module thesis.
- Indicate the most useful next inspection paths through visual emphasis, not a FAQ.
- Treat spatial rings as a map legend, not as the primary task.

### 2. Ownership Boundary

Probe: "What meaning belongs inside this module, and what must not leak out?"

Current behavior:

- The ownership ring lists concepts, data, identities, and decisions.
- Labels include `Payment`, `Authorization`, `Capture`, `Refund`, `payment-ledger`, `payment-id`, and eligibility decisions.

Gap:

- Owned items have generic summaries such as "Owned concepts in the payments module."
- The atlas does not distinguish domain meaning, private state, public identity, decision authority, or implementation detail.
- It does not show what would violate the boundary.

Interaction outcome:

- Ownership mode should group concepts, identities, private state, and decisions.
- Selecting an owned item should show the ownership implication.
- Public/private/boundary-crossing meaning should be visually and textually distinct.

### 3. Public Surface

Probe: "What can other modules call, query, or subscribe to?"

Current behavior:

- The public-surface layer lists commands, query, event, and boundary.
- Selecting `capture-payment` shows a short summary and contract path.

Observed screenshot:

- `dist/payments-atlas/audit/03-capture-payment.png`

Gap:

- The selected command only connects back to `payments`.
- Preconditions, outcomes, failure semantics, emitted events, and consumers are not composed into the interaction.
- The user must open the contract manually to understand behavior.

Interaction outcome:

- Public-surface focus should reveal contract, meaning, preconditions, result/failure semantics, related invariants, effects, state, and verification.
- This can be expressed as a focused inspection state, not as a visible written question.

### 4. Change Risk

Probe: "Can I safely change `capture-payment`?"

Current behavior:

- Search finds `capture-payment`.
- The inspector shows "Request capture against an active authorization."
- Source refs include `payments.module.yaml` and `contracts/capture-payment.v1.yaml`.

Gap:

- The atlas does not compose the change-risk trace.
- It does not show that capture is constrained by `capture-requires-authorization`.
- It does not show payment provider effect, state lifecycle, compatibility, or proof obligations.
- It gives source paths but no change workflow.

Interaction outcome:

- Change mode should start from a selected public surface and highlight the contract, constraints, effects, state, compatibility policy, and verification evidence that shape the change.
- The side panel should read like a change checklist, not a graph description.

### 5. External Truth And Recovery

Probe: "Where does this module touch the outside world, and what happens when the outside world disagrees?"

Current behavior:

- The effects layer shows `payment-provider`.
- The effect summary includes `external-financial-operation`, `payment-operation-id`, `per-payment`, `external-eventual`, and `unknown-outcome`.

Observed screenshot:

- `dist/payments-atlas/audit/04-effect.png`

Gap:

- Valuable operational semantics are present but not explained through interaction.
- The effect is connected only to the module, not to public operations or recovery paths.
- The user cannot tell which operations may produce unknown provider outcomes.

Interaction outcome:

- Operate/debug mode should treat effects as operational risk.
- Idempotency, ordering, consistency, timeout, and reconciliation should be explained in the selected focus state.
- Unproved trigger links should be shown as gaps, not invented.

### 6. Proof And Confidence

Probe: "What proves this module keeps its promises?"

Current behavior:

- The verification layer shows evidence nodes.
- Selecting `capture_requires_authorization` shows "Evidence artifact referenced by the module."
- The source path is visible.

Observed screenshot:

- `dist/payments-atlas/audit/05-proof.png`

Gap:

- Evidence is shown as a file path, not as proof of a promise.
- The UI does not distinguish law, contract, scenario, boundary, or adapter evidence in the primary flow.
- Evidence strength, currentness, and gaps are not visible.

Interaction outcome:

- Verify mode should organize evidence by the promise being protected.
- Selecting proof should show the invariant or contract, the evidence lane, source path, and remaining gaps.

### 7. Debugging Journey

Probe: "A payment is stuck after provider timeout; where should I look first?"

Current behavior:

- The atlas can expose provider effect, state model, payment status query, and verification files independently.

Gap:

- There is no incident-oriented flow.
- Timeout semantics are not connected to state, reconciliation, query surface, or evidence.
- The map leaves the user to invent the debugging path.

Interaction outcome:

- Debug mode should start from symptom families: timeout, duplicate operation, impossible transition, missing event, stale query, consumer breakage.
- A timeout path should trace effect semantics, idempotency key, ordering boundary, state model, reconciliation path, observable query, and evidence.

### 8. Composition And Dependencies

Probe: "What does this module require from other modules or capabilities?"

Current behavior:

- The dependency layer lists `payment-provider` and `durable-store`.
- Source contracts are present for required capabilities.

Gap:

- Dependency direction is not explained.
- Capabilities are not framed as assumptions the module relies on.
- There is no composition-readiness view.

Interaction outcome:

- Compose mode should separate required modules from required capabilities.
- Each requirement should state what the module assumes, which contract declares it, and what happens if it is unavailable.

### 9. Compatibility Review

Probe: "What would break if this public contract changes?"

Current behavior:

- The compatibility policy node exists: `backward-compatible-within-major`.
- Public contract paths are visible.

Gap:

- Compatibility is not tied to any public surface.
- There is no breaking-change or migration view.
- The atlas cannot help assess a proposed contract change.

Interaction outcome:

- Change mode should show compatibility policy near every public surface.
- Compatibility-sensitive areas should include input shape, result shape, event payload, failure semantics, ordering, idempotency, persisted state, and migration policy.

### 10. Live Iteration

Probe: "I changed the manifest or contract. What changed in the atlas?"

Current behavior:

- `atlas.json` declares `supports_live_reconciliation: true`.
- The generated app is static. It does not show diffs or reload generated data.

Gap:

- The user cannot see semantic deltas while iterating.
- Added, changed, and removed semantic IDs are not surfaced.
- There is no confidence that the visual stayed synced with canonical artifacts.

Interaction outcome:

- Iteration mode should support regeneration, reload, and semantic diff.
- Changed nodes and traces should be highlighted.
- Removed semantic IDs should be visible as removed, not silently missing.

## Refactor Direction

A refactor is justified if the atlas is meant to make understanding easy. The current implementation can be polished, but the main failure is structural:

- The document model has `nodes`, `edges`, `layers`, and `tours`; it does not express journey states, trace reasons, focus priorities, or gaps.
- Graph edges mostly connect module-to-item. They do not explain why a selected item matters in a maintainer workflow.
- The inspector exposes adjacency. It should become a focus panel for the current inspection state.
- Search filters nodes. It should support object lookup and intent-shaped routing without turning the UI into a chat box.
- Tour advances through nodes. It should advance through understanding tasks.

## Proposed Atlas Shape

Keep the spatial map, but make it secondary to guided inspection.

Data model additions:

- `journeys`: deterministic inspection modes such as Understand, Change, Verify, Debug, Compose, Iterate.
- `focus_states`: generated states for important semantic objects and tasks.
- `traces`: ordered semantic traces containing node IDs, edge IDs, reasons, and source refs.
- `gaps`: missing or ambiguous evidence that prevents confident interpretation.
- `next_actions`: suggested commands, source refs, or follow-up inspections.

Interaction shape:

```yaml
id: change-public-surface
mode: Change
entry_node_kinds:
  - public-surface
focus_blocks:
  - kind: contract
  - kind: constraints
  - kind: effects
  - kind: compatibility
  - kind: proof
trace_roles:
  - contract
  - invariant
  - effect
  - state-model
  - verification
```

UI shape:

- Left or top: compact mode controls for Understand, Change, Verify, Debug, Compose, Iterate.
- Center: 2.5D semantic map with pan, zoom, rotate, and trace highlights.
- Right: focus panel with selected meaning, why it matters, trace reasons, sources, gaps, and next action.
- Search: object lookup first; intent-aware routing only when it improves navigation.
- Bottom: live status and semantic diff when watch mode is active.

The map should remain abstract and calm. No gates, buildings, roads, or ornamental metaphor are needed. The right spatial metaphor is topology: center, owned interior, public edge, outside dependencies, external effects, proof layer, and highlighted traces.

## Agentic Generation Policy

Agent generation is useful for microcopy, journey labels, and trace explanations, but topology must remain deterministic.

Allowed:

- Generate explanatory copy from existing canonical artifacts.
- Add focus states only when every referenced semantic ID exists in `nodes`.
- Add next actions from a fixed allowlist of intents and RMS commands.
- Mark uncertainty as a gap instead of inventing a link.

Not allowed:

- Invent nodes, dependencies, effects, invariants, consumers, or evidence.
- Hide missing artifacts behind prose.
- Change source refs or topology without canonical artifact support.

## Verification Plan

Add fixture coverage for at least these interaction contracts:

- A public command focus state includes its contract source.
- A command constrained by an invariant includes that invariant in its change-risk trace.
- An external effect focus state exposes idempotency, ordering, consistency, and timeout semantics.
- A proof focus state names the promise being proved, not only the evidence path.
- A watch-mode or diff fixture marks added, removed, and changed semantic IDs.

Browser verification should check:

- A user can understand the change risk of `capture-payment` without interpreting raw edge lists.
- Selecting a trace changes the map highlight.
- Search can find `capture-payment` and route it into the appropriate focus state.
- The focus panel remains readable at desktop and mobile widths.

## Done Criteria

The atlas refactor is done when:

- Every primary journey has a deterministic focus state or an explicit gap.
- No primary workflow requires reading a raw adjacency list.
- Every focus state has source refs.
- Trace highlights explain why each highlighted node matters.
- The app supports fast iteration through reload, watch mode, or semantic diff.
- The public `build-module-atlas` contract is updated only after the implementation satisfies the new behavior.

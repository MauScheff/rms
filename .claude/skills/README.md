# RMS Agent Skills

## Authority

Canonical RMS manifests, contracts, applied revisions, and evidence declarations own system semantics. These skills are operational workflows: they select deterministic CLI context and proof lanes without becoming architectural authority.

Product intent is sufficient input. Apply semantic changes before code, edit only declared roles, and treat provider output as advisory.

## Catalog

- `inspect-module`: establish ownership, boundaries, contracts, effects, and evidence before planning.
- `implement-change`: implement a feature or fix inside declared RMS roles.
- `refactor-module`: improve internal structure without changing declared public meaning.
- `prune-module`: remove unnecessary implementation residue while preserving obligations.
- `add-module`: design and add a coherent module or recursive capability.
- `evolve-contract`: change commands, queries, events, APIs, schemas, or failure semantics.
- `compose-modules`: verify provider/consumer fit and cross-module behavior.
- `verify-module`: prove declared laws, contracts, boundaries, profiles, and compatibility promises.
- `hunt-bugs`: run resumable proof, fuzz, sanitizer, schedule, and mutation campaigns; minimize and replay every behavioral failure.

## Doorway

For work that requests or may require a software change, start with:

```text
rms next "<exact change task>" --root . --ai
```

Use native project tools for read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, and discussion that requests no change. If that work reveals a proposed change, stop before editing and enter this doorway with the exact change task.

Typed intent is reserved for CI, offline, or intentionally pre-structured caller input. Do not synthesize it as an automatic fallback when provider execution fails. A non-ready or ownerless route selects no owner; candidates and context are evidence, not permission to imply one.

RMS records schema-constrained fact extraction without accepting provider topology. CI, offline, and intentionally pre-structured callers use `--intent-json`, `--intent-yaml`, or `--intent-file`. Pass the returned `run_id`, run directory, or receipt file through `--route-receipt` on prescribed canonical semantic and topology mutations, including dry-runs.

Use `rms explain ["<question>"]` when the compact prescription needs clarification and `rms help --all` for specialist commands. Load the catalog skill selected by the task lane; detailed machine, surface, property, trace, and completion rules live in selected skills and rendered RMS context.

## Executable Property Loop

Behavioral contracts and temporal promises use typed observations, explicit assumptions, closed expressions, and dimensionally valid quantities. Use `property check` to type-check, `evaluate` for real invocation or transition records, `search` for a witness or counterexample, `analyze` for finite or solver-backed obligations, `monitor` for a fail-open streaming prefix, and `replay` for recorded `rms/property-analysis/v0.2` evidence. A bounded or open run is inconclusive, never proof.

For a transition-count bound, declare the metric observation exactly as `source: {kind: trace-metric, name: transition-count}` with `value: {quantity: transition}`; put `{metric: <observation-id>, value: <decimal>, unit: transition}` in the temporal bound. `rms spec plan` renders a complete bounded-response example for other agents and dimensions.

For finite machine-, protocol-, resource-, artifact-, or composition-scope proof, use `strategy: deterministic-exhaustive` with `exhaustive: true`. Without the explicit flag RMS treats the realization as non-exhaustive.

## Proof-First Bug Hunting

Derive strong lanes from risk when adding or changing software: generated or exhaustive checks for pure and numeric decisions; finite exploration for machines and workflows; coverage fuzzing for untrusted boundaries; schedule and fault exploration for distributed behavior; analyzers or sanitizers for unsafe authority; mutation testing for important reusable oracles; and real-trace evaluation plus violation search for temporal promises. Preserve historical counterexamples as smoke replays.

Fast commit checks require the appropriate lane or a focused `verification.hunt_exceptions` reason, but do not run overnight work. Use `rms hunt --root . --dry-run` to inspect the campaign and `rms hunt --root . --budget 8h` from a clean commit to run it. `clean-under-recorded-bounds` is bounded evidence, not a bug-free guarantee.

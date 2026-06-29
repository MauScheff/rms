---
name: implement-change
description: Implement a feature, fix, or refactor in an RMS project while preserving module ownership, contracts, effects, compatibility, and focused verification.
---

# Implement a Change

1. Run the `inspect-module` workflow for the owning module. When the CLI is available, use `rms implement <module> --task "<task>"` to render a bounded implementation prompt before editing when that would help.
2. Use `rms route <module> --task "<task>"` first when the target may be a composite parent or recursive module tree. Build a bounded packet with `rms context <module> --task "<task>"` when the CLI is available. Use `rms design --root <root> --task "<task>"` when module boundaries or semantic shapes are unclear, and always do this in a fresh project created from product intent only before choosing the first module tree. Use `rms plan <module> --task "<task>"` when a planning prompt would help before editing.
3. Restate the requested outcome in the owning context's domain language.
4. Classify the change:
   - private implementation;
   - invariant or domain-policy change;
   - public-contract change;
   - new dependency or effect;
   - state or migration change;
   - workflow change.
5. Define semantic structure before code:
   - closed variants or ADTs;
   - validated values;
   - commands, states, events, and accepted/rejected result types;
   - command, event, effect, and effect-result envelopes;
   - transition output, transition records, journal, timeline projection, replay bundle, and first-bad-transition evidence;
   - transition boundaries;
   - parser, port, adapter, trace, and evidence roles.
6. Resolve semantic edge cases before implementation:
   - invalid commands;
   - impossible variants;
   - invalid constructors;
   - malformed boundary input;
   - illegal transitions;
   - terminal-state behavior;
   - stale or conflicting state;
   - duplicate or out-of-order external facts;
   - expected effect failures.
7. Update the public contract or manifest first when public meaning changes.
8. Before implementing, decide whether the task requires scope expansion or a module split. If it does, update canonical artifacts before deepening the current module.
9. Implement inside the owning boundary.
10. Preserve or strengthen the module's representation:
   - use algebraic data types, sealed variants, or enums for closed domain alternatives;
   - use opaque types and validated constructors for values with validity rules;
   - use explicit result types for expected domain failures;
   - use schemas and validators at untrusted or versioned boundaries;
   - use query/projector-produced read models for derived facts; if such public types have private fields and no public constructor, declare them in `architecture.allowed_missing_constructors` and add evidence for the producing query/projector;
   - use a state model or transition function only when behavior depends on lifecycle order.
11. Keep representation, pure transitions, boundary parsing, ports/adapters, and trace/evidence roles separate where practical. Use domain-named role suffixes where the language allows it: `<Domain>Machine`, `<Domain>State`, `<Domain>Command`, `<Domain>Event`, `<Domain>Effect`, `<Domain>EffectResult`, `<Domain>Reply`, and `<Domain>Rejection`. Do not derive inner names from role or surface suffixes such as rules, engine, adapter, cli, web, rust, swift, or js unless those words are genuine domain language. Workflows orchestrate, machines execute, commands ask, events report, effects touch the world, projections observe, journals explain, replay reproduces, and first-bad-transition evidence points to the fix. Replace generated role files incrementally; do not delete a declared role file and leave the project invalid while hand-building a replacement. When replacing generated role code, update `implementation.yaml` in the same change so `architecture.roles`, `architecture.machine`, `architecture.representation`, and `semantic_functions` name the actual files and symbols.
12. When a change touches lifecycle behavior, update the declared state model before implementation and make illegal transitions rejected or unrepresentable.
13. Keep decisions separate from external effects where practical.
14. Do not introduce undeclared dependencies, effects, or cross-module state mutation. Keep projections passive: they may derive facts and timelines from observed inputs, but they must not emit workflow commands or mutate another module's state.
15. Add the smallest verification evidence that demonstrates:
   - affected laws;
   - contract compatibility;
   - meaningful success and failure scenarios;
   - impossible variants, invalid constructors, and illegal transitions when applicable;
   - boundary behavior when applicable;
   - transition records, golden timelines, replay bundles, and first-bad-transition diagnostics for stateful or workflow behavior.
16. Run `rms review <module>` before finalizing when a diff exists. Run `rms validate --root <root>`, `rms structure <implementation.yaml>` when inner roles changed, and project-native verification from the implementation binding. Use `rms verify <implementation.yaml>` when the binding declares `commands.verify`, or `rms verify <composite-module.yaml>` for composite rollups. Do not declare implemented modules done while validation reports `evidence.placeholder`, `evidence.bootstrap-active`, `evidence.source-unpinned`, or `evidence.semantic-shape-only` for those modules.
17. Summarize:
    - changed behavior;
    - affected contracts and invariants;
    - representation choices for ADTs, constructors, results, schemas, or state machines;
    - compatibility impact;
    - new effects or dependencies;
    - verification evidence;
    - operational or migration notes.

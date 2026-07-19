---
name: refactor-module
description: Refactor an RMS module's internal structure while preserving public contracts, ownership, effects, compatibility, and verification evidence.
---

# Refactor an RMS Module

Use this skill when the requested outcome is better internal shape, clearer boundaries, stronger representation, or lower accidental complexity without intended public behavior change.

1. Run the `inspect-module` workflow for the owning module.
2. Build a bounded packet with `rms context <module> --task "<task>"` when the CLI is available. Use `rms refactor <module> --task "<task>"` when a refactor prompt would help before editing.
3. Treat RMS as the semantic and architecture gate. Apply structural revisions through `rms spec apply --dry-run --route-receipt <RUN_ID>` or focused machine/surface apply with the same ready receipt, inspect the complete final model, and let RMS hash-seal and chain the semantic revision. Never repair scaffold drift by hand-editing canonical manifests or deleting applied change records.
4. State the public semantics that must be preserved:
   - public commands, queries, events, APIs, and capabilities;
   - invariants and laws;
   - declared effects and required capabilities;
   - profiles and operational semantics;
   - compatibility policy and active consumers.
   - exact public contract-to-function-to-machine bindings and required capability consumer-to-provider bindings.
   - artifact contracts and transformations, cross-module protocol automata, resource lifecycles, authority containment, and temporal properties.
5. Classify the refactor target:
   - unclear ownership or misplaced concept;
   - weak domain representation;
   - accidental abstraction or duplication;
   - decision/effect coupling;
   - public/private boundary leakage;
   - state model, transition, or lifecycle clutter;
   - verification gap.
6. Improve representation without changing meaning:
   - closed domain alternatives: ADTs, sealed variants, or enums;
   - values with validity rules: opaque types, validated constructors, or smart constructors;
   - expected domain failures: explicit result types;
   - untrusted or versioned input: schemas and boundary validators;
   - query/projector-produced read models: private fields without public constructors only when `architecture.allowed_missing_constructors` and evidence name the producing query/projector;
   - lifecycle/order-dependent behavior: state model, transition table, or transition function.
7. Do not add a state machine merely because a record has a status field. Use one only when legal behavior depends on lifecycle or order.
8. Keep domain decisions separate from effects where practical. Move IO, clocks, randomness, storage, network, and vendor calls behind declared effects or capabilities. Private helpers inside pure role files must stay pure.
   - Keep type mappings separate from semantic alternatives.
   - Stateful machines use one input ADT and one `transition(state, input)` path for commands, observed events, and effect results.
   - Every transition branch has a stable semantic case mirrored by replay source provenance. Declared cases occur in declared transition source, source-only branches are first added through RMS semantics, every lifecycle state is reachable from `initial_state`, and provenance names the actual transition source file.
   - Effectful stateful machines declare exact driver and transition-record functions, and each effect protocol declares an exact executor symbol plus an effectful `effect-executor` semantic function. Runnable effect paths reach the driver; the driver stores complete records and owns the complete repeated transition/effect/result cycle. Move output-only histories, outer loops around one-step drivers, retry, compensation, and stop/continue policy out of surfaces, adapters, and executors and into the machine path, even when public and machine command names differ.
   - Preserve one public protocol automaton across modules, close every resource on terminal paths, keep elevated operations behind declared safe facades, and retain proof strategies that match temporal scope.
9. Preserve module boundaries:
   - do not move private state across ownership boundaries;
   - do not expose private implementation as public contract;
   - do not put context-specific business concepts into the technical kernel;
   - do not introduce undeclared dependencies or effects.
   - do not move real product behavior into an undeclared runnable surface that bypasses the declared public entrypoint, parser, adapter, or boundary machine, and do not duplicate parser, generator, transition, or domain decisions inside runnable entrypoints or browser launch scripts.
   - do not keep generic `Accept`/`Reject` scaffold commands as product semantics when public commands are domain-specific.
10. If public meaning must change to complete the refactor, stop treating the work as a private refactor. Switch to `evolve-contract` or `implement-change` and make compatibility impact explicit.
11. Add or adjust focused verification evidence:
   - laws and invariants still hold;
   - impossible variants and invalid constructors are rejected or unrepresentable;
   - illegal state transitions are rejected or unrepresentable;
   - boundary validation still rejects malformed input;
   - public contract behavior remains compatible.
12. Run `rms review <module>` before finalizing when a diff exists. Run `rms validate --root <root>`, `rms spec check <module.yaml|implementation.yaml>`, `rms machine check <implementation.yaml>`, `rms surface check <implementation.yaml> --strict` when runnable surfaces exist, and `rms structure <implementation.yaml>` when implementation structure exists, plus the implementation binding's build and verification commands. Use `rms check-compat` when public manifests changed. Treat public-command representation and runnable-surface diagnostics as refactor blockers unless the canonical artifacts declare a real exception.
13. Summarize:
    - preserved public semantics;
    - internal representation changes;
    - boundary and dependency impact;
    - verification evidence;
    - any residual risk or follow-up contract work.

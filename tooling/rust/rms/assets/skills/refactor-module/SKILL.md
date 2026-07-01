---
name: refactor-module
description: Refactor an RMS module's internal structure while preserving public contracts, ownership, effects, compatibility, and verification evidence.
---

# Refactor an RMS Module

Use this skill when the requested outcome is better internal shape, clearer boundaries, stronger representation, or lower accidental complexity without intended public behavior change.

1. Run the `inspect-module` workflow for the owning module.
2. Build a bounded packet with `rms context <module> --task "<task>"` when the CLI is available. Use `rms refactor <module> --task "<task>"` when a refactor prompt would help before editing.
3. Treat RMS as the semantic and architecture gate. If the refactor requires new laws, contracts, states, commands, events, effects, effect results, transitions, semantic roles, public entrypoints, or evidence obligations, produce and apply an `rms/semantic-change/v0.1` object with `rms spec apply` before editing role bodies. Use `rms machine apply` only for focused inner-machine changes after semantics are already correct.
4. State the public semantics that must be preserved:
   - public commands, queries, events, APIs, and capabilities;
   - invariants and laws;
   - declared effects and required capabilities;
   - profiles and operational semantics;
   - compatibility policy and active consumers.
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
9. Preserve module boundaries:
   - do not move private state across ownership boundaries;
   - do not expose private implementation as public contract;
   - do not put context-specific business concepts into the technical kernel;
   - do not introduce undeclared dependencies or effects.
10. If public meaning must change to complete the refactor, stop treating the work as a private refactor. Switch to `evolve-contract` or `implement-change` and make compatibility impact explicit.
11. Add or adjust focused verification evidence:
   - laws and invariants still hold;
   - impossible variants and invalid constructors are rejected or unrepresentable;
   - illegal state transitions are rejected or unrepresentable;
   - boundary validation still rejects malformed input;
   - public contract behavior remains compatible.
12. Run `rms review <module>` before finalizing when a diff exists. Run `rms validate --root <root>`, `rms spec check <module.yaml|implementation.yaml>`, `rms machine check <implementation.yaml>` when implementation structure exists, and the implementation binding's build and verification commands. Use `rms check-compat` when public manifests changed.
13. Summarize:
    - preserved public semantics;
    - internal representation changes;
    - boundary and dependency impact;
    - verification evidence;
    - any residual risk or follow-up contract work.

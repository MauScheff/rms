---
name: implement-change
description: Implement a feature, fix, or refactor in an RMS project while preserving module ownership, contracts, effects, compatibility, and focused verification.
---

# Implement a Change

1. Run the `inspect-module` workflow for the owning module. When the CLI is available, use `rms implement <module> --task "<task>"` to render a bounded implementation prompt before editing when that would help.
2. Use `rms route <module> --task "<task>"` first when the target may be a composite parent or recursive module tree. Build a bounded packet with `rms context <module> --task "<task>"` when the CLI is available. Use `rms design --root <root> --task "<task>"` when module boundaries or semantic shapes are unclear, and `rms plan <module> --task "<task>"` when a planning prompt would help before editing.
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
11. Keep representation, pure transitions, boundary parsing, ports/adapters, and trace/evidence roles separate where practical.
12. When a change touches lifecycle behavior, update the declared state model before implementation and make illegal transitions rejected or unrepresentable.
13. Keep decisions separate from external effects where practical.
14. Do not introduce undeclared dependencies, effects, or cross-module state mutation.
15. Add the smallest verification evidence that demonstrates:
   - affected laws;
   - contract compatibility;
   - meaningful success and failure scenarios;
   - impossible variants, invalid constructors, and illegal transitions when applicable;
   - boundary behavior when applicable.
16. Run `rms review <module>` before finalizing when a diff exists. Run `rms validate --root <root>` and project-native verification from the implementation binding. Use `rms verify <implementation.yaml>` when the binding declares `commands.verify`, or `rms verify <composite-module.yaml>` for composite rollups.
17. Summarize:
    - changed behavior;
    - affected contracts and invariants;
    - representation choices for ADTs, constructors, results, schemas, or state machines;
    - compatibility impact;
    - new effects or dependencies;
    - verification evidence;
    - operational or migration notes.

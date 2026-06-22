---
name: implement-change
description: Implement a feature, fix, or refactor in an RMS project while preserving module ownership, contracts, effects, compatibility, and focused verification.
---

# Implement a Change

1. Run the `inspect-module` workflow for the owning module. When the CLI is available, use `rms implement <module> --task "<task>"` to render a bounded implementation prompt before editing when that would help.
2. Build a bounded packet with `rms context <module> --task "<task>"` when the CLI is available. Use `rms plan <module> --task "<task>"` when a planning prompt would help before editing.
3. Restate the requested outcome in the owning context's domain language.
4. Classify the change:
   - private implementation;
   - invariant or domain-policy change;
   - public-contract change;
   - new dependency or effect;
   - state or migration change;
   - workflow change.
5. Update the public contract or manifest first when public meaning changes.
6. Implement inside the owning boundary.
7. Preserve or strengthen the module's domain representation:
   - use algebraic data types, sealed variants, or enums for closed domain alternatives;
   - use opaque types and validated constructors for values with validity rules;
   - use explicit result types for expected domain failures;
   - use schemas and validators at untrusted or versioned boundaries;
   - use a state model or transition function only when behavior depends on lifecycle order.
8. When a change touches lifecycle behavior, update the declared state model before implementation and make illegal transitions rejected or unrepresentable.
9. Keep decisions separate from external effects where practical.
10. Do not introduce undeclared dependencies, effects, or cross-module state mutation.
11. Add the smallest verification evidence that demonstrates:
   - affected laws;
   - contract compatibility;
   - meaningful success and failure scenarios;
   - impossible variants, invalid constructors, and illegal transitions when applicable;
   - boundary behavior when applicable.
12. Run `rms review <module>` before finalizing when a diff exists. Run `rms validate --root <root>` and project-native verification from the implementation binding. Use `rms verify <implementation.yaml>` when the binding declares `commands.verify`.
13. Summarize:
    - changed behavior;
    - affected contracts and invariants;
    - representation choices for ADTs, constructors, results, schemas, or state machines;
    - compatibility impact;
    - new effects or dependencies;
    - verification evidence;
    - operational or migration notes.

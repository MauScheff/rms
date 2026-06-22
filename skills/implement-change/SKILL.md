---
name: implement-change
description: Implement a feature, fix, or refactor in an RMS project while preserving module ownership, contracts, effects, compatibility, and focused verification.
---

# Implement a Change

1. Run the `inspect-module` workflow for the owning module.
2. Restate the requested outcome in the owning context's domain language.
3. Classify the change:
   - private implementation;
   - invariant or domain-policy change;
   - public-contract change;
   - new dependency or effect;
   - state or migration change;
   - workflow change.
4. Update the public contract or manifest first when public meaning changes.
5. Implement inside the owning boundary.
6. Preserve or strengthen the module's domain representation:
   - use algebraic data types, sealed variants, or enums for closed domain alternatives;
   - use opaque types and validated constructors for values with validity rules;
   - use explicit result types for expected domain failures;
   - use schemas and validators at untrusted or versioned boundaries;
   - use a state model or transition function only when behavior depends on lifecycle order.
7. When a change touches lifecycle behavior, update the declared state model before implementation and make illegal transitions rejected or unrepresentable.
8. Keep decisions separate from external effects where practical.
9. Do not introduce undeclared dependencies, effects, or cross-module state mutation.
10. Add the smallest verification evidence that demonstrates:
   - affected laws;
   - contract compatibility;
   - meaningful success and failure scenarios;
   - impossible variants, invalid constructors, and illegal transitions when applicable;
   - boundary behavior when applicable.
11. Run project-native validation and verification from the implementation binding.
12. Summarize:
    - changed behavior;
    - affected contracts and invariants;
    - representation choices for ADTs, constructors, results, schemas, or state machines;
    - compatibility impact;
    - new effects or dependencies;
    - verification evidence;
    - operational or migration notes.

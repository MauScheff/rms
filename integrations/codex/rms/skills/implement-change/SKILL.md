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
6. Keep decisions separate from external effects where practical.
7. Do not introduce undeclared dependencies, effects, or cross-module state mutation.
8. Add the smallest verification evidence that demonstrates:
   - affected laws;
   - contract compatibility;
   - meaningful success and failure scenarios;
   - boundary behavior when applicable.
9. Run project-native validation and verification from the implementation binding.
10. Summarize:
    - changed behavior;
    - affected contracts and invariants;
    - compatibility impact;
    - new effects or dependencies;
    - verification evidence;
    - operational or migration notes.

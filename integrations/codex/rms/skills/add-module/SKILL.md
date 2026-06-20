---
name: add-module
description: Design and add a new RMS module or bounded context with a coherent purpose, ownership boundary, public contract, dependencies, profiles, and conformance evidence.
---

# Add a Module

1. Confirm that a new boundary is justified by distinct language, ownership, invariants, change cadence, or replaceability. Do not create a module for every noun.
2. Choose whether the new unit is a bounded context, internal module, workflow, adapter, or library.
3. Define:
   - one-sentence purpose;
   - authoritative terminology;
   - owned concepts, data, identities, and decisions;
   - public commands, queries, events, APIs, or capabilities;
   - required capabilities and contracts;
   - important invariants;
   - declared effects;
   - compatibility policy.
4. Declare the Core profile and only the additional profiles that actually apply.
5. Define the consistency boundary and lifecycle only if the Stateful profile applies.
6. Define retry, timeout, ordering, duplicate handling, and reconciliation only if the Distributed profile applies.
7. Define coordination state, deadlines, terminal states, and compensation only if the Workflow profile applies.
8. Add law, contract, and scenario evidence. Add boundary evidence only for exposed or untrusted input.
9. Update the system manifest, context map, and glossary index.
10. Validate that no existing owner now has overlapping responsibility.

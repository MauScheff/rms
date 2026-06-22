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
5. Choose the strongest domain representation the implementation language supports:
   - closed sets of domain meaning: algebraic data types, sealed variants, or enums;
   - values with validity rules: opaque types, validated constructors, or smart constructors;
   - expected domain failures: explicit result types rather than ambient exceptions;
   - untrusted or versioned input: runtime schemas and boundary validators;
   - lifecycle/order-dependent behavior: a state model, transition table, or state machine.
6. Do not introduce a state machine only because a record has a status field. Use one when legal commands, facts, or invariants depend on lifecycle order.
7. Define the consistency boundary, lifecycle, legal transitions, concurrency, persistence, and migration only if the Stateful profile applies. Illegal transitions must be rejected or made unrepresentable.
8. Define retry, timeout, ordering, duplicate handling, and reconciliation only if the Distributed profile applies.
9. Define coordination state, deadlines, terminal states, and compensation only if the Workflow profile applies.
10. Add law, contract, and scenario evidence. Include negative evidence for impossible variants, invalid constructors, malformed boundary input, and illegal transitions when applicable.
11. Update the system manifest, context map, and glossary index.
12. Validate that no existing owner now has overlapping responsibility.

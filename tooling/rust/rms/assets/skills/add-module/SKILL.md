---
name: add-module
description: Design and add a new RMS module or bounded context with a coherent purpose, ownership boundary, public contract, dependencies, profiles, and conformance evidence.
---

# Add a Module

1. Run `rms diagnose` when starting from an unfamiliar checkout.
2. Confirm that a new boundary is justified by distinct language, ownership, invariants, change cadence, or replaceability. Do not create a module for every noun.
3. Use `rms add-module <path> --name <name> --purpose "<purpose>"` when the CLI can scaffold the requested shape. Add `--binding rust` or `--binding swift` for static language bindings, and `--binding executable` for opaque command-backed surfaces such as web, mobile, CLI, native UI, generated assets, or integration scripts. Refine the generated artifacts rather than maintaining parallel agent-only scaffolding.
4. Choose whether the new unit is a bounded context, internal module, workflow, adapter, or library.
5. Define:
   - one-sentence purpose;
   - authoritative terminology;
   - owned concepts, data, identities, and decisions;
   - public commands, queries, events, APIs, or capabilities;
   - required capabilities and contracts;
   - important invariants;
   - declared effects;
   - compatibility policy.
6. Declare the Core profile and only the additional profiles that actually apply.
7. Choose the strongest domain representation the implementation language supports:
   - closed sets of domain meaning: algebraic data types, sealed variants, or enums;
   - values with validity rules: opaque types, validated constructors, or smart constructors;
   - expected domain failures: explicit result types rather than ambient exceptions;
   - untrusted or versioned input: runtime schemas and boundary validators;
   - query/projector output: read models or result structs may omit public constructors only when the implementation binding declares `architecture.allowed_missing_constructors` and evidence names the producing query/projector;
   - lifecycle/order-dependent behavior: a state model, transition table, or state machine.
8. Do not introduce a state machine only because a record has a status field. Use one when legal commands, facts, or invariants depend on lifecycle order.
9. Define the consistency boundary, lifecycle, legal transitions, concurrency, persistence, and migration only if the Stateful profile applies. Illegal transitions must be rejected or made unrepresentable.
10. Define retry, timeout, ordering, duplicate handling, and reconciliation only if the Distributed profile applies.
11. Define coordination state, deadlines, terminal states, and compensation only if the Workflow profile applies.
12. Add law, contract, and scenario evidence. Include negative evidence for impossible variants, invalid constructors, malformed boundary input, and illegal transitions when applicable.
13. Update the system manifest, context map, and glossary index.
14. Validate that no existing owner now has overlapping responsibility with `rms validate --root <root>` and `rms compose --root <root>`.

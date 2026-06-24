---
name: add-module
description: Design and add a new RMS module or bounded context with a coherent purpose, ownership boundary, public contract, dependencies, profiles, and conformance evidence.
---

# Add a Module

1. Run `rms diagnose` when starting from an unfamiliar checkout.
2. Confirm that a new boundary is justified by distinct language, ownership, invariants, change cadence, or replaceability. Do not create a module for every noun.
3. Use `rms design --root <root> --task "<task>"` first when module boundaries or semantic shapes are unclear.
4. Use `rms add-capability <path> --name <name> --purpose "<purpose>"` when the requested capability should be a recursive tree with a composite parent, domain child, and boundary child. Use `rms add-module <path> --name <name> --purpose "<purpose>"` when the CLI can scaffold one requested shape. Use `--shape domain-engine`, `--shape boundary-adapter`, `--shape workflow`, `--shape storage-adapter`, `--shape integration-adapter`, or `--shape composite` to record semantic role obligations. Add `--binding rust`, `--binding swift`, `--binding js`, or `--binding executable` for implementation scaffolding. Refine the generated artifacts rather than maintaining parallel agent-only scaffolding.
5. Choose whether the new unit is a bounded context, internal module, workflow, adapter, or library.
6. Define:
   - one-sentence purpose;
   - authoritative terminology;
   - owned concepts, data, identities, and decisions;
   - public commands, queries, events, APIs, or capabilities;
   - required capabilities and contracts;
   - important invariants;
   - declared effects;
   - compatibility policy.
7. Declare the Core profile and only the additional profiles that actually apply.
8. Choose the strongest representation the implementation language supports:
   - closed sets of domain meaning: algebraic data types, sealed variants, or enums;
   - values with validity rules: opaque types, validated constructors, or smart constructors;
   - expected domain failures: explicit result types rather than ambient exceptions;
   - untrusted or versioned input: runtime schemas and boundary validators;
   - query/projector output: read models or result structs may omit public constructors only when the implementation binding declares `architecture.allowed_missing_constructors` and evidence names the producing query/projector;
   - lifecycle/order-dependent behavior: a state model, transition table, or state machine.
9. Treat representation as a first-class role: closed variants, validated values, commands, states, events, result/rejection types, and boundary schemas belong in an explicit language-idiomatic unit.
10. Keep pure transitions separate from representation definitions, and keep boundary parsing separate from both.
11. Do not introduce a state machine only because a record has a status field. Use one when legal commands, facts, or invariants depend on lifecycle order.
12. Define the consistency boundary, lifecycle, legal transitions, concurrency, persistence, and migration only if the Stateful profile applies. Illegal transitions must be rejected or made unrepresentable.
13. Define retry, timeout, ordering, duplicate handling, and reconciliation only if the Distributed profile applies.
14. Define coordination state, deadlines, terminal states, and compensation only if the Workflow profile applies.
15. Add law, contract, and scenario evidence. Include negative evidence for impossible variants, invalid constructors, malformed boundary input, illegal transitions, and replay traces when applicable.
16. Update the system manifest, context map, and glossary index.
17. Validate that no existing owner now has overlapping responsibility with `rms validate --root <root>` and `rms compose --root <root>`.

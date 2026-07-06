---
name: add-module
description: Design and add a new RMS module or bounded context with a coherent purpose, ownership boundary, public contract, dependencies, profiles, and conformance evidence.
---

# Add a Module

1. Run `rms diagnose` when starting from an unfamiliar checkout.
2. Confirm that a new boundary is justified by distinct language, ownership, invariants, change cadence, or replaceability. Do not create a module for every noun.
3. Use `rms design --root <root> --task "<task>"` first when module boundaries or semantic shapes are unclear. In a fresh project created from product intent only, treat boundaries as unclear until design has proposed the module tree.
4. Use `rms add-capability <path> --name <name> --purpose "<purpose>"` when the requested capability should be a recursive tree with a composite parent, domain child, and boundary child, especially when the intent combines user/boundary interaction, lifecycle decisions, and effect simulation or external-service coordination. Omit `--domain-child` and `--boundary-child` unless the user supplied semantic child names; do not invent `-rules`, `-engine`, `-adapter`, `-cli`, `-web`, or binding-specific child names just to describe roles. For app, tool, CLI, local-first reference app, runnable, or smoke-test intents, declare a runnable surface through RMS unless the user explicitly requested a library-only package. Use `rms add-module <path> --name <name> --purpose "<purpose>"` when the CLI can scaffold one requested shape. Use `--shape domain-engine`, `--shape boundary-adapter`, `--shape runtime-monitor`, `--shape workflow`, `--shape storage-adapter`, `--shape integration-adapter`, or `--shape composite` to record semantic role obligations. Add `--binding rust`, `--binding swift`, `--binding js`, or `--binding executable` for implementation scaffolding. Refine the generated artifacts rather than maintaining parallel agent-only scaffolding.
5. Choose whether the new unit is a bounded context, internal module, runtime monitor, workflow, adapter, or library.
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
9. Treat representation as a first-class role: closed variants, validated values, commands, states, events, result/rejection types, message envelopes, transition outputs, transition records, and boundary schemas belong in explicit language-idiomatic units.
10. Keep pure transitions separate from representation definitions, and keep boundary parsing separate from both. For traceable machines, transitions return next state, emitted events, commands, effects, and reply; replay evidence records state before, state after, input, outputs, and source branch. When replacing generated role code, update `implementation.yaml` in the same change so `architecture.roles`, `architecture.machine`, `architecture.representation`, and `semantic_functions` name the actual files and symbols.
11. Do not introduce a state machine only because a record has a status field. Use one when legal commands, facts, or invariants depend on lifecycle order. After scaffolding, use `rms spec apply` for new laws, contracts, machine variants, transitions, semantic roles, public entrypoints, and evidence obligations rather than hand-creating architecture files. Use `set`, `remove`, and `supersedes` to revise RMS semantics instead of editing manifests or old change records by hand. Use `rms machine apply` only for focused inner-machine edits after the semantic layer is correct.
12. Public commands in `module.yaml` must be represented by the declared implementation machine, parser, adapter, transition, representation, or semantic functions. Do not leave generated `Accept`/`Reject` command scaffolds as the real product semantics when the module publishes product-specific commands.
13. Runnable app, UI, browser, CLI, HTTP, batch, and executable surfaces must be declared with `rms surface apply` or `rms spec apply` and must route through the declared public entrypoint, parser, adapter, or boundary machine. For browser surfaces, distinguish the inspectable controller `entrypoint` from the host `launch_entrypoint`; any local script loaded by the host is also part of the surface and must import/call the declared controller or adapter, not duplicate parser/generator/transition/domain logic in a second bundle. Simple runnable surfaces should stay thin and stateless unless lifecycle/order/session/retry/status/recovery/workflow semantics are real. Do not put real behavior only in `public/app.*`, `src/cli.*`, routes, mobile views, or similar files while the RMS boundary remains disconnected.
14. Define the consistency boundary, lifecycle, legal transitions, concurrency, persistence, and migration only if the Stateful profile applies. Illegal transitions must be rejected or made unrepresentable.
15. Define retry, timeout, ordering, duplicate handling, and reconciliation only if the Distributed profile applies.
16. Define coordination state, deadlines, terminal states, subscription registry, effect lifecycle, retry count, golden timelines, and compensation only if the Workflow profile applies.
17. Define observed inputs, derived facts or streams, trigger conditions, monitor authority, retrigger/idempotency policy, and failure mode only if the Monitor profile applies.
18. Add law, contract, scenario, boundary, and runtime evidence as applicable. Include negative evidence for impossible variants, invalid constructors, malformed boundary input, illegal transitions, transition records, replay bundles, first-bad-transition diagnostics, and monitor trigger/non-trigger cases when applicable. Replace scaffold placeholder evidence before declaring the module implemented; `rms validate --root <root>` should not report `evidence.placeholder`, `evidence.bootstrap-active`, `evidence.source-unpinned`, or `evidence.semantic-shape-only` for implemented modules.
19. Update the system manifest, context map, and glossary index.
20. Validate that no existing owner now has overlapping responsibility with `rms validate --root <root>` and `rms compose --root <root>`.

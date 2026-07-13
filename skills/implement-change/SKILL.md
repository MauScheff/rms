---
name: implement-change
description: Implement a feature, fix, or refactor in an RMS project while preserving module ownership, contracts, effects, compatibility, and focused verification.
---

# Implement a Change

1. Run the `inspect-module` workflow for the owning module. When the CLI is available, use `rms implement <module> --task "<task>"` to render a bounded implementation prompt before editing when that would help.
2. Use `rms route <module> --task "<task>"` first when the target may be a composite parent or recursive module tree. Build a bounded packet with `rms context <module> --task "<task>"` when the CLI is available. Use `rms design --root <root> --task "<task>"` when module boundaries or semantic shapes are unclear, and always do this in a fresh project created from product intent only before choosing the first module tree. Use `rms plan <module> --task "<task>"` when a planning prompt would help before editing.
3. Treat RMS as the semantic and architecture gate. If the change needs new laws, contracts, states, commands, events, effects, effect results, replies, rejections, transitions, semantic roles, runnable surfaces, public entrypoints, properties, fuzz targets, or evidence obligations, run `rms spec plan`, apply the complete `rms/semantic-change/v0.1` object with `rms spec apply --dry-run`, inspect `final_machine`, then apply and run `rms spec check`. Do not proceed while product semantics remain generic scaffold cases. RMS apply records the exact change and seals the canonical semantic revision; direct manifest edits after apply are drift. Focused `rms machine apply/check` is only for structure after laws, contracts, properties, and evidence obligations are correct. It preserves evidence roles but does not generate replay proof; update and replay those roles after implementation. Use `rms surface apply/check` for runnable entrypoints.
   - Never repair canonical manifests, contracts, roles, surfaces, or evidence declarations by direct editing. If RMS cannot express the required semantic change, stop and report the RMS gap instead of bypassing the gate.
   - In a fresh project, follow the deterministic `rms design` scaffold recommendation. Do not collapse a recommended recursive capability into one module unless explicit user intent and a canonical single-module justification permit it.
4. Restate the requested outcome in the owning context's domain language.
5. Classify the change:
   - private implementation;
   - invariant or domain-policy change;
   - public-contract change;
   - new dependency or effect;
   - state or migration change;
   - workflow change.
6. Define semantic structure before code:
   - closed variants or ADTs;
   - validated values;
   - commands, states, events, and accepted/rejected result types;
   - command, event, effect, and effect-result envelopes;
   - transition output, transition records, journal, timeline projection, replay bundle, and first-bad-transition evidence;
   - semantic properties with input spaces, preconditions, operations, oracles, evidence, and counterexample replay policy for broad laws;
   - fuzz-style targets for parser, boundary, generated-input, numeric, or external-result surfaces where malformed or adversarial input matters;
   - transition boundaries;
   - parser, port, adapter, trace, and evidence roles.
   - binding type mappings separately from semantic alternatives;
   - one classified input ADT over commands, observed events, and effect results for stateful machines;
   - one-request-one-result effect protocols whenever outcomes can alter subsequent decisions.
7. Resolve semantic edge cases before implementation:
   - invalid commands;
   - impossible variants;
   - invalid constructors;
   - malformed boundary input;
   - illegal transitions;
   - terminal-state behavior;
   - stale or conflicting state;
   - duplicate or out-of-order external facts;
   - expected effect failures.
8. Update the public contract or manifest first when public meaning changes.
9. Before implementing, decide whether the task requires scope expansion or a module split. If it does, update canonical artifacts before deepening the current module.
10. Implement inside the owning boundary and inside RMS-declared role files. Small private helpers inside pure role files must stay pure; IO belongs in declared adapter, port, or effect-executor roles as effects plus effect results.
    - Do not implement real product behavior only in an undeclared runnable surface while the declared machine remains generic.
    - Public commands in `module.yaml` must be represented in the declared machine, parser, adapter, transition, representation, or semantic functions.
    - Runnable surfaces must delegate to an existing declared role or concrete symbol, declare boundary effects or a precise no-effect justification, and route through the parser/adapter/boundary before pure decisions.
    - Reusable modules expose meaning through RMS capabilities/contracts and one declared public facade. Consumers must import the facade or call a contract-shaped entrypoint, not private `representation`, `transition`, parser, adapter, or port role files. Native package manifests are binding evidence only.
    - Generic `Accept`/`Reject` scaffold commands are not done when the module publishes product-specific commands.
11. Preserve or strengthen the module's representation:
   - use algebraic data types, sealed variants, or enums for closed domain alternatives;
   - use opaque types and validated constructors for values with validity rules;
   - use explicit result types for expected domain failures;
   - use schemas and validators at untrusted or versioned boundaries;
   - use query/projector-produced read models for derived facts; if such public types have private fields and no public constructor, declare them in `architecture.allowed_missing_constructors` and add evidence for the producing query/projector;
   - use a state model or transition function only when behavior depends on lifecycle order.
12. Keep representation, pure transitions, boundary parsing, ports/adapters, and trace/evidence roles separate. Stateful machines dispatch every command, observed event, and effect result through one `transition(state, input)` function. Every branch has a stable transition `case`, and trace source branches use the same name. Effect executors perform one request and return one result; transitions own iteration, retry, compensation, stop/continue policy, and progress. Public domain values keep private fields and validated constructors; do not use `allowed_public_field_structs` to exempt them.
13. When a change touches lifecycle behavior, update laws/contracts/evidence plus the declared state model through RMS spec apply before implementation and make illegal transitions rejected or unrepresentable.
14. Keep decisions separate from external effects where practical.
15. Do not introduce undeclared dependencies, effects, or cross-module state mutation. Keep projections passive: they may derive facts and timelines from observed inputs, but they must not emit workflow commands or mutate another module's state.
16. Add the smallest verification evidence that demonstrates:
   - affected laws;
   - contract compatibility;
   - meaningful success and failure scenarios;
   - impossible variants, invalid constructors, and illegal transitions when applicable;
   - boundary behavior when applicable;
   - property obligations for always/never/bounded/ordered/normalized/parser/numeric laws, including generated input spaces and oracles;
   - replayable counterexamples for failed generated cases;
   - transition records, golden timelines, replay bundles, and first-bad-transition diagnostics for stateful or workflow behavior.
17. Run `rms property check <module.yaml|implementation.yaml>` whenever properties, fuzz targets, parsers, numeric bounds, reusable modules, or generated counterexamples are involved. Run `rms property run <implementation.yaml> --profile smoke` when the binding declares property or fuzz commands, and `rms property replay <counterexample.yaml>` for recorded failures.
18. For reusable modules, run `rms package <module.yaml>` and `rms verify-package <package-dir>` before claiming the module can be reused outside its owner.
19. Run `rms review <module>` before finalizing when a diff exists. Run `rms validate --root <root>`, `rms spec check <module.yaml|implementation.yaml>`, `rms machine check <implementation.yaml>`, `rms property check <module.yaml|implementation.yaml>`, `rms surface check <implementation.yaml> --strict` when runnable surfaces exist, `rms structure <implementation.yaml>` when inner roles changed, and project-native verification from the implementation binding. Use `rms verify <implementation.yaml>` when the binding declares `commands.verify`, or `rms verify <composite-module.yaml>` for composite rollups. Treat `semantic.contract-scaffold-active`, `structure.public-command-not-represented`, `structure.generic-scaffold-command-active`, `structure.native-package-export-mismatch`, `semantic.reusable-capability-missing`, `semantic.reusable-package-evidence-missing`, `semantic.property-without-input-space`, `semantic.property-without-oracle`, `structure.property-target-missing`, `structure.boundary-parser-without-fuzz-property`, and `structure.runnable-surface-*` as architecture-gate failures, not cleanup suggestions. Do not declare implemented modules done while validation reports `evidence.placeholder`, `evidence.bootstrap-active`, `evidence.source-unpinned`, or `evidence.semantic-shape-only` for those modules. Evidence must not claim a current filesystem snapshot or missing Git revision; strict audit resolves the committed candidate revision.
    - Completion is binary: `rms gate --root <root>` must exit zero before the candidate commit, and `rms audit --root <root> --strict` must exit zero after it. A failed check is not a manual note, and `review-required` never justifies simplifying away recommended structure.
20. Summarize:
    - changed behavior;
    - affected contracts and invariants;
    - representation choices for ADTs, constructors, results, schemas, or state machines;
    - compatibility impact;
    - new effects or dependencies;
    - verification evidence;
    - operational or migration notes.

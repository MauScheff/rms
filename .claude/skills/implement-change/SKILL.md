---
name: implement-change
description: Implement a feature, fix, or refactor in an RMS project while preserving module ownership, contracts, effects, compatibility, and focused verification.
---

# Implement a Change

1. Run the `inspect-module` workflow for the owning module. When the CLI is available, use `rms implement <module> --task "<task>"` to render a bounded implementation prompt before editing when that would help.
2. For the requested software change, begin with `rms next "<exact change task>" --root . --ai` for recorded typed extraction and ownership routing. This skill does not apply to read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, or discussion that requests no change; use native project tools for those tasks. If read-only work reveals a proposed change, stop before editing and begin this workflow with the exact change task. Use typed intent flags only for CI, offline, or intentionally pre-structured caller input, never as an automatic provider-failure fallback. If the route is non-ready or ownerless, stop the owner-scoped workflow: do not infer an owner from candidates, context, neighboring modules, or implementation language. Obtain an explicit caller decision, model or adopt the boundary, or state that the work is outside RMS coverage. Use `rms design` only when module boundaries change; never infer topology from raw task wording. Build a bounded packet with `rms context`, attach an intentionally deferred implementation with `rms add-binding --route-receipt <RUN_ID>`, and use `rms plan` only after RMS has selected the owner in a ready route.
3. Treat RMS as the semantic and architecture gate. If the change needs new laws, contracts, states, commands, events, effects, effect results, replies, rejections, transitions, semantic roles, runnable surfaces, public entrypoints, properties, fuzz targets, behavior bindings, or evidence obligations, run `rms spec plan`, apply the complete `rms/semantic-change/v0.1` object with `rms spec apply --dry-run`, and inspect its final machine, semantic functions, public behavior bindings, and dependency behavior bindings. Every public behavior closes contract -> discharging semantic function -> classified machine cases -> proof; every required capability closes exact consumer -> matching provider contract or explicit external boundary. Then apply and run `rms spec check`. Do not proceed while product semantics remain generic scaffold cases or either chain is unresolved. Spec apply records and hash-seals the exact change and automatically closes every active semantic revision; never edit or delete applied records. Direct manifest edits after apply are drift. Focused `rms machine apply/check` is only for structure after laws, contracts, properties, and evidence obligations are correct. It preserves evidence roles but does not generate replay proof; update and replay those roles after implementation. Use `rms surface apply/check` for runnable entrypoints.
   - Never repair canonical manifests, contracts, roles, semantic-function bindings, surfaces, or evidence declarations by direct editing. Use `semantic_functions.add/set/remove` for authority owners, exact symbols, purity, discharged promises, assumptions, and evidence. If RMS cannot express the required semantic change, stop and report the RMS gap instead of bypassing the gate.
   - Follow the deterministic typed-design scaffold action exactly. Pure reusable libraries are normal standalone modules; only mixed runnable implementations use `unsplit_runnable_justification`.
   - Treat the rendered RMS plan schema as self-contained. Use `set` to replace generic scaffold semantics and `add`/`remove` for incremental changes; keep `surfaces.set: null` unless replacing the complete runnable surface set, and remove intentional surfaces explicitly by name. Do not inspect sibling projects, prior dogfood runs, RMS source, or generated examples outside the project to infer the schema or borrow semantics.
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
   - fuzz-style targets for parser, boundary, generated-input, numeric, or external-result surfaces where malformed or adversarial input matters; every realization names an exact runner, generated or exhaustive strategies name a generator, and the runner executes the semantic operation and oracle;
   - transition boundaries;
   - parser, port, adapter, trace, and evidence roles.
   - binding type mappings separately from semantic alternatives;
   - one classified input ADT over commands, observed events, and effect results for stateful machines;
   - one-request-one-result effect protocols whenever outcomes can alter subsequent decisions.
   - execution-derived trace producers that call the real transition-record path and serialize returned records rather than copying declarations.
   - versioned artifacts and transformations when data changes form or crosses a module boundary;
   - public protocol automata for ordered cross-module conversations;
   - resource ownership automata for acquire/use/release/transfer lifecycles;
   - exact safe facades for privileged, unsafe, or foreign authority;
   - temporal properties and scope-appropriate realizations for always, eventually, ordering, exclusion, at-most-once, and bounded-response claims.
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
    - Shared effectful mechanics used by multiple exact executors belong in a declared private `effect_support` role. Keep business sequencing, state construction, transitions, drivers, and runnable/public behavior out of that role.
    - Do not implement real product behavior only in an undeclared runnable surface while the declared machine remains generic.
    - Public commands in `module.yaml` must be represented in the declared machine, parser, adapter, transition, representation, or semantic functions.
    - Runnable surfaces must delegate to an existing declared role or concrete symbol, declare boundary effects or a precise no-effect justification, name a concrete usage document and implementation smoke command, and route through the parser/adapter/boundary before pure decisions.
    - Reusable modules expose meaning through RMS capabilities/contracts and one declared public facade. Consumers must import the facade or call a contract-shaped entrypoint, not private `representation`, `transition`, parser, adapter, or port role files. Native package manifests are binding evidence only.
    - Generic `Accept`/`Reject` scaffold commands are not done when the module publishes product-specific commands.
11. Preserve or strengthen the module's representation:
   - use algebraic data types, sealed variants, or enums for closed domain alternatives;
   - use opaque types and validated constructors for values with validity rules;
   - use explicit result types for expected domain failures;
   - use schemas and validators at untrusted or versioned boundaries;
   - use query/projector-produced read models for derived facts; if such public types have private fields and no public constructor, declare them in `architecture.allowed_missing_constructors` and add evidence for the producing query/projector;
   - use a state model or transition function only when behavior depends on lifecycle order.
12. Keep representation, pure transitions, boundary parsing, machine drivers, effect executors, and trace/evidence roles separate. Stateful machines dispatch every command, observed event, and effect result through one `transition(state, input)` function. Every branch has a stable transition `case`; every declared case occurs in the declared transition source, no source-only branch invents semantics, and every lifecycle state is reachable from `initial_state`. Expected failures remain in the transition's typed `rejection` channel rather than replies, dummy values, status strings, or provenance labels. Trace provenance names the transition source file and exact case, not a trace YAML file, and every trace record must match that case's exact state change, events, commands, effects, reply, and rejection. Generate records from the real transition path; copied declaration lists are not execution evidence. An effectful stateful machine declares exact `driver_function` and `transition_record_function` callables. The driver calls the record function, retains complete records, advances from `state_after`, and executes only `output.effects`; output-only history is not diagnostic evidence. Effect protocols declare exact `executor_symbol` functions and matching effectful `effect-executor` semantic functions; each executor role names its exact effect and uses a dedicated path separate from transition and machine-driver code. The execution path is runnable callable -> machine driver -> pure transition record -> one-request executor -> typed effect result -> machine driver. The driver owns the whole repeated cycle until reply, rejection, or a declared waiting state; a surface must not loop around a one-step driver, even when its public command name differs from the machine command. Inspectable boundary IO is also an explicit effect protocol with typed results and a dedicated executor; runnable delegation names an exact callable, not merely a file. Every declared message envelope is represented in the binding. Arithmetic over represented transition inputs is checked or bounded and returns explicit rejection for extreme values. Transitions own iteration, retry, compensation, stop/continue policy, and progress; surfaces, adapters, and executors do not hide a second lifecycle loop. Public domain values keep private fields and validated constructors; do not use `allowed_public_field_structs` to exempt them.
    - While implementing or debugging a machine, use `rms probe <implementation.yaml> --describe`, then `rms probe <implementation.yaml> --input '<JSON>'` or a scenario file to inspect the real transition-record path. Supply effect results explicitly; never use a probe as permission to execute effects or as a substitute for declared proof.
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
   - artifact compatibility and transformation preservation;
   - protocol composition plus stitched cross-module causation;
   - resource closure on every reachable terminal path;
   - authority containment behind the declared safe facade;
   - temporal properties using exhaustive, model-checking, static-analysis, sanitizer, or benchmark evidence appropriate to their scope.
17. Run `rms property check <module.yaml|implementation.yaml>` whenever properties, fuzz targets, parsers, numeric bounds, reusable modules, or generated counterexamples are involved. Run `rms property run <implementation.yaml> --profile smoke` when the binding declares property or fuzz commands, and `rms property replay <counterexample.yaml>` for recorded failures.
18. For reusable modules, run `rms package <module.yaml>` before claiming reuse. It builds, verifies, records the concrete result in declared package evidence, rebuilds, and verifies the final artifact. Use `rms verify-package <package-dir>` for an independent recheck; expected-result prose alone is not package proof.
19. Run `rms review <module>` before finalizing when a diff exists. Run `rms validate --root <root>`, `rms spec check <module.yaml|implementation.yaml>`, `rms machine check <implementation.yaml>`, `rms property check <module.yaml|implementation.yaml>`, `rms surface check <implementation.yaml> --strict` when runnable surfaces exist, `rms structure <implementation.yaml>` when inner roles changed, and project-native verification from the implementation binding. Use `rms verify <implementation.yaml>` when the binding declares `commands.verify`, or `rms verify <composite-module.yaml>` for composite rollups. Treat `semantic.contract-scaffold-active`, `structure.public-command-not-represented`, `structure.generic-scaffold-command-active`, `structure.native-package-export-mismatch`, `semantic.reusable-capability-missing`, `semantic.reusable-package-evidence-missing`, `semantic.property-without-input-space`, `semantic.property-without-oracle`, `structure.property-target-missing`, `structure.boundary-parser-without-fuzz-property`, and `structure.runnable-surface-*` as architecture-gate failures, not cleanup suggestions. Do not declare implemented modules done while validation reports `evidence.placeholder`, `evidence.bootstrap-active`, `evidence.source-unpinned`, or `evidence.semantic-shape-only` for those modules. Evidence must not claim a current filesystem snapshot or missing Git revision; strict audit resolves the committed candidate revision.
    - Completion is binary: `rms check --changes --root <root>` must exit zero before an authorized candidate commit, and `rms check --committed --root <root>` must exit zero after it. A failed check is not a manual note, and `review-required` never justifies simplifying away recommended structure.
20. Summarize:
    - changed behavior;
    - affected contracts and invariants;
    - representation choices for ADTs, constructors, results, schemas, or state machines;
    - compatibility impact;
    - new effects or dependencies;
    - verification evidence;
    - operational or migration notes.

# Agent Instructions

This repository follows Reliable Modular Systems.

RMS artifacts are the architectural source of truth. Do not infer ownership, effects, dependencies, compatibility, or recovery behavior from incidental code shape when manifests or contracts say otherwise.

Product intent is enough input from the user. Convert natural language into RMS semantics by asking only necessary clarifying questions, surfacing edge cases, naming what must never happen, and encoding the result in canonical artifacts before code.

Core rule:

- RMS owns semantics and architecture.
- Agents fill declared roles.
- Bugs should become diagnosable bad states.

## Non-Negotiable Execution

1. In a fresh standalone project, run `rms init`, then commit the generated bootstrap so later semantic and source drift has a provenance baseline.
2. Run `rms design` before choosing the first module tree. When its deterministic hints recommend a recursive capability, use `rms add-capability`; choose explicit implementation bindings for work that will produce code. Omit binding flags only for an intentionally semantic-only scaffold, and use `rms add-binding` before machine or surface work if a binding was deferred; do not substitute one module for convenience.
3. Never repair `module.yaml`, `implementation.yaml`, contracts, machine structure, surfaces, or evidence declarations by direct editing. Use the applicable RMS apply command. If RMS cannot express the required change, stop and report the RMS gap instead of bypassing the gate.
4. Use the current project, rendered RMS prompts, and deterministic RMS diagnostics as planning context. Do not inspect sibling projects, prior dogfood runs, RMS source, or generated examples outside the project to infer a change schema or borrow semantics.
5. Fill only declared role bodies after the semantic apply succeeds.
6. Before completion, `rms gate --root .` must exit zero with no failed check. A warning or `review-required` item is an obligation to resolve or report, never permission to collapse the recommended architecture.
7. Commit the candidate, then `rms audit --root . --strict` must exit zero. Do not describe the project as complete or production-ready unless both commands succeeded in that order.

## Change Gate

| Change | Required RMS gate before source edits |
| --- | --- |
| Meaning, law, contract, property, artifact, protocol, resource, authority, effect, evidence | `rms spec plan/apply/check` |
| State, command, observed event, effect result, transition | `rms spec apply` or focused `rms machine apply/check` |
| App, CLI, UI, HTTP, batch, executable entrypoint | `rms surface apply/check` |
| Module boundary or public capability | `rms design` then `rms add-module` or `rms add-capability` |
| Implementation realization for a semantic-only module | `rms add-binding <module.yaml> --binding <binding>` |
| Local implementation dependency | `rms spec apply` with language-neutral `binding_dependencies` |
| Semantic function owner, symbol, purity, or discharged promise | `rms spec apply` with `semantic_functions.add/set/remove` |
| Declared role body only | Edit the role body, then verify |

Machine rules:

- `architecture.machine.types` names binding containers; semantic lists name actual cases.
- Stateful, boundary, workflow, storage, integration, and projection machines use `transition(state, input)`.
- The input ADT closes over commands, observed events, and effect results; each case belongs to exactly one category.
- Every canonical transition declares a stable `case`, and every implemented transition branch is declared. Distinct outcomes for the same state/input are separate named cases; declaration-only and source-only branches are drift.
- Every declared lifecycle state is reachable from `initial_state` through canonical transitions. Do not make a state look covered by starting a trace inside an otherwise unreachable state.
- Replay provenance names the declared transition role source file and the exact canonical case. An evidence YAML file is not transition source code.
- Transition outputs carry expected failures in an explicit typed `rejection` channel. Do not erase rejection variants into replies, status strings, dummy success values, or provenance branch names.
- Replay records must match the canonical case's state change and exact events, commands, effects, reply, and rejection. Do not hand-author active trace records. A declared trace producer must call the real transition-record path, serialize the returned records, and regenerate the committed bundle through `rms trace run --record`.
- An effect executor performs one declared request and returns one declared result. Each exact protocol `executor_symbol` names its effect in the RMS role declaration and is bound as an effectful `effect-executor` semantic function. Keep its role path separate from transition and machine-driver code. Transitions own sequencing, retry, compensation, stop/continue policy, and state progression.
- Atomicity belongs to each exact effect protocol. Aggregate iteration requires aggregate justification and evidence; shared IO mechanics belong in a private `effect_support` role that cannot construct machine state, call transitions/drivers, or become public/runnable.
- Every effectful stateful machine declares exact `driver_function` and `transition_record_function` callables. The driver retains complete transition records, advances from `state_after`, executes `output.effects`, and feeds typed results back as inputs.
- Every declared command, event, effect, and effect-result envelope has a binding-native type or tagged constructor; declarations without binding representation are drift.
- Arithmetic over represented transition inputs, including indices, counters, attempts, offsets, lengths, and sequence values, is checked or bounded so extreme inputs become explicit rejection rather than overflow, panic, or trap.
- Every effect-emitting runnable surface delegates to an exact callable that reaches the machine driver. The driver owns the complete repeated transition/effect/result cycle until reply, rejection, or a declared waiting state; surfaces and adapters must not loop around a one-step driver, even when public and machine command names differ.
- Inspectable boundary IO is a declared effect protocol with typed results and a dedicated executor. Do not perform filesystem, process, network, clock, randomness, or persistence work directly inside a parser, pure transition, runnable controller, or undecorated adapter helper.
- Runnable surface delegation names an exact callable, not only a role or source file, so RMS can prove the live app reaches the declared machine.
- Every runnable surface declares a concrete `usage_document` and an implementation `smoke_command` key; `rms verify` executes the resolved smoke command.
- Composite parent laws may use `verification.delegations` only when the contained provider, provider law/property, public export, and concrete evidence all resolve.
- Fixed examples are a deterministic corpus, not an open-ended fuzz realization. A property generator constructs cases from the declared input space; its exact runner executes the semantic operation and oracle. Every realization names a `path#symbol` runner, and generated or exhaustive strategies also name a `path#symbol` generator.
- Property evidence emitted by `rms spec apply` is an obligation, not proof. Replace it with the exact executed command, observed result, and counterexample or coverage summary before production audit. Revise property semantics through `properties.set/remove`, not direct manifest edits.
- Public cross-module conversations use contract protocol automata. Each participant is bound once, each message has one sender and receiver mapping, and system traces preserve envelope identity, correlation, and causation across the handoff.
- Versioned artifacts declare provided, required, or internal contracts. Transformations name input/output artifacts, explicit rejections, exact semantic functions, and preserving properties.
- Resources whose lifetime affects correctness declare ownership and acquire/use/release/transfer automata. Every reachable terminal product path closes or transfers each resource.
- Privileged, unsafe, and foreign operations occur only in declared authority roles behind exact safe facade symbols and evidence.
- Always, eventually, precedence, exclusion, at-most-once, bounded-response, and resource-closure claims are temporal properties with scope-appropriate realizations.

You can:

- edit bodies inside RMS-declared role files;
- add small private pure helpers inside declared pure role files;
- add effectful helper code only inside declared adapter, port, or effect-executor roles;
- import another module only through its declared public facade or contract-shaped entrypoint;
- use provider-backed RMS prompts as advisory planning input.

You cannot:

- hand-create laws, contracts, artifacts, transformations, protocol/resource automata, authority boundaries, public commands, states, events, effects, effect results, transitions, semantic roles, semantic-function bindings, runnable surfaces, public entrypoints, or evidence obligations;
- implement real product behavior only in an undeclared runnable surface while the declared machine remains generic;
- bypass another module's public contract or a module's declared public entrypoint;
- import another module's private role files such as representation, transition, parser, adapter internals, or native package exports that bypass the RMS public facade;
- treat provider output, generated reports, or command logs as semantic authority until RMS canonical artifacts reflect them.
- treat generated package, build, dependency, or atlas output as live project semantics; author canonical artifacts only in source-owned module paths.

## Before Changing Behavior

1. Run `rms diagnose`.
2. Identify the owning module for the requested behavior.
3. Run `rms explain <module.yaml>` to understand ownership, public surface, effects, invariants, compatibility, and verification evidence.
4. Run `rms route <module.yaml> --task "<task>"` when the target may be a composite parent or recursive module tree.
5. Run `rms context <module.yaml> --task "<task>"` before implementation work.
6. Read the target `module.yaml`, public contracts, direct dependency contracts, applicable glossary entries, and implementation binding.

Use these advisory workbench commands when they match the task:

- Fresh intent-only project: after `rms init`, run `rms design --root . --task "<task>"` before choosing `rms add-module` or `rms add-capability`.
- Implemented project: pass `--binding` to `rms add-module`, or both `--domain-binding` and `--boundary-binding` to `rms add-capability`; use `rms add-binding` only when an intentionally semantic-only module later gains code.
- `rms design --root . --task "<task>"` before module boundaries or semantic shapes are fixed
- `rms route <module.yaml> --task "<task>"` before implementing against a composite parent
- `rms plan <module.yaml> --task "<task>"`
- `rms implement <module.yaml> --task "<task>"`
- `rms evolve-contract <module.yaml> --task "<task>"`
- `rms evidence <module.yaml> --task "<task>"`
- `rms refactor <module.yaml> --task "<task>"`
- `rms spec plan <module.yaml|implementation.yaml> --task "<task>"` when a change needs new laws, contracts, artifacts, transformations, protocols, resource lifecycles, authority boundaries, temporal properties, states, commands, events, effects, effect results, replies, rejections, transitions, semantic roles, semantic-function bindings, public entrypoints, binding dependencies, or evidence obligations
- `rms spec apply <module.yaml|implementation.yaml> --change-json '<json>'` or `--change-yaml '<yaml>'` to update canonical semantics, record and hash-seal the exact applied change, and automatically supersede every currently active semantic revision; use `semantic_functions.add/set/remove` for exact binding symbols, authority owners, purity, discharged promises, and evidence; `binding_dependencies` names RMS modules and lets the binding adapter realize native dependency metadata; use `contracts.set` with `direction: provided|required` to replace generated provider or consumer contract scaffolds without transferring ownership; use `set`, `remove`, and explicit `supersedes` only for additional non-local branches instead of hand-editing manifests or old change records; provider output is advisory until this succeeds
- `rms spec check <module.yaml|implementation.yaml>` after semantic changes
- `rms machine plan/apply/check <implementation.yaml>` only for focused inner-machine edits after laws, contracts, and evidence obligations are already correct
- `rms surface apply/check <implementation.yaml>` when adding or changing app, UI, CLI, browser, HTTP, batch, or executable entrypoints; browser-style surfaces should distinguish controller `entrypoint` from host `launch_entrypoint`, and declare intentional local launch scripts with `--launch-script`
- In semantic-change objects, keep `surfaces.set: null` when surfaces are unchanged. Remove intentional surfaces by name; an empty replacement is rejected because it can erase runnable entrypoints accidentally.
- `rms structure <implementation.yaml>` when implementation inner roles, machine declarations, or evidence placeholders are unclear
- `rms review <module.yaml> --impact`

Provider-backed prompts are opt-in. Use `--provider codex` or `--ai` only when an external Codex run is intended.

## Adding Modules

When creating a new capability, choose semantic shape before file layout:

- `domain-engine`: pure decisions, closed variants, validated values, transition records, laws, and replay bundles.
- `boundary-adapter`: parsers, boundary validation, ports, effect adapters, and boundary/contract tests.
- `runtime-monitor`: observed runtime inputs, derived facts or streams, trigger decisions, monitor authority, and runtime evidence.
- `workflow`: commands, states, events, deadlines, compensation, recovery evidence.
- `storage-adapter`: persistence ports, failure categories, migration and recovery evidence.
- `integration-adapter`: external service boundary, retries, idempotency, reconciliation evidence.
- `composite`: contained submodules, public exports, visibility boundaries, composition evidence.

Use `rms add-capability <path> --name <name> --purpose "<purpose>" --domain-binding <binding> --boundary-binding <binding>` when a public capability should be implemented as a recursive tree with a composite parent, domain child, and boundary child. Prefer this over a single module when the intent combines a runnable surface or untrusted input with invariant-bearing planning, ordering, batching, filtering, policy, lifecycle decisions, or external effects. Omit bindings only when the requested output is deliberately semantic-only; attach them later with `rms add-binding <child>/module.yaml --binding <binding>` rather than copying a scratch scaffold.

If the user intent says app, tool, CLI, local-first reference app, runnable, or smoke test, declare a runnable surface through RMS. A library-only boundary is acceptable only when the product intent is explicitly library-only. Runnable surfaces stay thin, but boundary machines still use explicit state-plus-input transitions; product lifecycle belongs in the owning domain or workflow machine.

If a pure/domain module is meant to be reused like a library or Lego block, declare the reusable capability in `provides.capabilities[]`, keep a single public code facade in `implementation.yaml`, and add package/reuse evidence. RMS says what is reusable; native package files only say how a binding imports it.

Use `rms add-module <path> --name <name> --purpose "<purpose>" --shape <shape> --binding <binding>` when scaffolding one module. Bindings realize semantic roles idiomatically; they do not define the semantics.

Default split for any capability: put invariant-bearing decisions in a `domain-engine`, and put untrusted input, output, UI, CLI, network, storage, time, randomness, external services, and other effects in adapters.

Naming rule: choose module and inner role names from product/capability language. When using `rms add-capability`, omit `--domain-child` and `--boundary-child` unless the user supplied semantic child names; the CLI defaults to neutral `-domain` and `-boundary` paths. Do not invent child names or machine names from role/surface words such as `rules`, `engine`, `adapter`, `cli`, `web`, `rust`, `swift`, or `js` unless those words are genuinely part of the domain language.

## Semantic Structure Before Code

Before writing implementation code, make the user's intent concrete enough to encode:

- Semantic gate: do not hand-create laws, contracts, semantic roles, semantic-function bindings, states, commands, events, effects, transition functions, parsers, runnable surfaces, public entrypoints, or evidence obligations. Use RMS CLI commands, especially `rms spec apply` and `rms surface apply`, then edit the declared role bodies. Use `semantic_functions.add/set/remove` when an authority owner, exact symbol, purity, discharged promise, or evidence binding changes. Use semantic `set` and `remove` operations instead of manual manifest surgery. `rms spec apply` automatically closes every currently active semantic revision; use explicit `supersedes` only for additional non-local branches. Never edit or delete an applied change record.
- Apply gate: run semantic or machine apply with `--dry-run` first. Do not write product code while `final_machine` still contains generic scaffold variants or omits real branches. Machine apply preserves evidence roles but does not generate replay proof; update and replay them from implemented paths. Direct edits after apply invalidate the semantic revision and strict audit.
- Public surface gate: generated capability contracts are scaffold obligations, not production semantics. Replace them through `rms spec apply` with `contracts.set` before implementation. Public commands in `module.yaml` must be represented by the declared implementation surface. A runnable surface adapts outside input into declared RMS commands, may render or execute declared boundary effects, and must not reimplement domain decisions or call private module internals. Generic `Accept`/`Reject` scaffold commands are not implemented product semantics.
- Reuse gate: reusable modules publish capabilities and contracts first and expose one declared public facade. `rms package` builds, verifies, records the concrete result, rebuilds, and verifies the final artifact; expected-result prose alone is not proof. Consumers must require the capability contract and import only the public facade.
- Property gate: laws that say always, never, bounded, ordered, normalized, parsed, generated, impossible, or must not happen should declare semantic properties with input spaces, oracles, evidence, and counterexample replay policy before relying on binding tests.
- Universal system questions: identify artifacts and transformations, ordered cross-module messages, resource ownership and closure, elevated authority, and always/eventually/bounded guarantees before choosing files.
- Intent: restate the behavior in the owning context's language and name what must never happen.
- ADTs and values: define closed variants, validated values, commands, states, events, and accepted/rejected result types.
- State and transitions: define accepted transitions, rejected transitions, terminal states, transition records, and replayable traces when behavior depends on order or lifecycle.
- Traceable machine: workflows orchestrate; machines execute; commands ask; events report; effects touch the world; projections observe; journals explain; replay reproduces; first-bad-transition evidence points to the fix.
- Messages and outputs: keep command, event, effect, and effect-result envelopes explicit; transition outputs should name next state, emitted events, commands, effects, and reply.
- Protocols: declare public multi-module conversations as closed participant/message/state automata, then bind each machine case as one send or receive.
- Resources: declare acquire/use/release/transfer protocols for files, handles, memory regions, transactions, locks, processes, devices, or any other lifetime-sensitive resource.
- Artifacts: declare versioned inputs/outputs and transformation contracts separately from in-memory machine state.
- Authority: contain privileged, unsafe, and foreign operations behind declared safe facades.
- Temporal proof: encode always, eventually, ordering, exclusion, at-most-once, and bounded-response claims as properties with a matching exhaustive, model-checking, static, sanitizer, or benchmark realization.
- Boundaries: parse untrusted input into domain commands before pure decisions, and keep external effects behind ports or adapters.
- Numeric safety: if validated values represent counts, money, quantities, rates, sizes, scores, or other numeric facts, choose checked, saturating, bounded, or explicitly proven arithmetic before implementation.
- Edge cases first: decide invalid commands, impossible variants, invalid constructors, malformed inputs, illegal transitions, stale or conflicting state, duplicate or out-of-order external facts, numeric overflow or rounding, and not-applicable cases.
- Property-first proof: convert broad laws into semantic properties; bindings may use native libraries or deterministic generated cases, but RMS owns the input space, oracle, evidence path, and replayable counterexample shape.
- External truth: decide what happens when an external outcome is unknown, duplicate, stale, partial, conflicting, delayed, or later corrected. Declare reconciliation, recovery, retry, compensation, or convergence evidence before relying on that behavior.
- Only then fill implementation files, tests, and evidence.

## While Implementing

- Keep changes inside the owning module boundary.
- Edit bodies inside RMS-declared role files. Add small private pure helpers inside declared pure role files when useful.
- Do not add private IO helpers in pure roles. Filesystem, network, clocks, randomness, environment, processes, provider calls, and external services must be declared effects with effect results and executed only in adapter, port, or effect-executor roles.
- For effectful stateful behavior, keep the execution chain explicit: runnable callable -> machine driver -> pure transition record -> exact one-request executor -> typed effect result -> machine driver. The driver retains complete records, advances from `state_after`, and owns the whole repeated cycle; do not put an outer lifecycle loop in the runnable surface.
- When new semantic structure is needed, run `rms spec plan/apply/check` instead of inventing files or naming schemes directly. Use `rms machine plan/apply/check` only for focused inner-machine edits after laws, contracts, and evidence obligations are already correct.
- Change public contracts or manifests before code when public meaning changes.
- Declare new effects, dependencies, profiles, state, migration, compatibility impact, and recovery paths before relying on them.
- Make representation first-class: closed variants, validated values, commands, states, events, and accepted/rejected result types belong in an explicit role or unit.
- Use domain-named role suffixes for generated or declared ADTs where the language allows it: `<Domain>Machine`, `<Domain>State`, `<Domain>Command`, `<Domain>Event`, `<Domain>Effect`, `<Domain>EffectResult`, `<Domain>Reply`, and `<Domain>Rejection`.
- Do not use role-derived inner names such as `<Domain>RulesMachine`, `<Domain>AdapterMachine`, `<Domain>CliMachine`, or `<Domain>WebMachine`; prefer `<Domain>Machine` for pure decisions and `<Domain>BoundaryMachine` only when a boundary state/transition role is useful.
- Keep pure transitions separate from representation definitions, and keep boundary parsing separate from both.
- Replace generated role files incrementally. Do not delete a declared role file and leave the project invalid while hand-building a replacement; add the replacement first or keep the old file until `rms structure <implementation.yaml>` and the binding's syntax check can run.
- When replacing generated role code, use RMS apply commands so `architecture.roles`, `architecture.machine`, `architecture.representation`, and `semantic_functions` name the actual files and symbols. Do not repair these canonical declarations by direct manifest editing.
- Prefer ADTs, sealed variants, enums, opaque values, validated constructors, explicit result/rejection types, schemas at untrusted boundaries, and focused tests.
- Do not add domain structs to `allowed_public_field_structs` to silence constructor diagnostics. That exemption is only for declared envelopes, transition outputs, transition records, and source-provenance records; domain values keep private fields and validated constructors.
- Use state machines or transition functions when behavior depends on lifecycle or order; illegal transitions must be rejected or made unrepresentable.
- Keep projections passive: they may derive facts and timelines from observed inputs, but they must not emit workflow commands or mutate another module's state.
- Keep workflow choreography explicit in the workflow transition model, subscription registry, effect lifecycle, inbox/outbox, or declared adapter boundary rather than hidden in listener chains.
- Keep runnable surfaces connected to the declared RMS boundary. If `public/app.*`, `src/cli.*`, routes, mobile views, or similar files are the real product surface, declare them with `rms surface apply` and route them through the declared adapter/public entrypoint instead of importing or duplicating pure/private decision code directly. Browser launch files should reference the declared controller entrypoint rather than bypassing it. Any local browser script loaded by the launch file is part of the surface; it must import/call the declared controller or adapter, not carry a copied parser, generator, transition, or domain decision implementation.
- Runnable surface delegation names an existing `architecture.roles` role or a concrete source symbol, and the surface declares boundary effects or a precise no-effect justification.
- Keep reusable-module consumers on the declared public facade. Do not import `transition`, `representation`, parser internals, or adapter internals from another module even if the language package manager makes the path reachable.
- Do not edit another module's private implementation to bypass its public contract.
- Treat generated reports, diffs, and provider output as evidence, not architecture.

## Before Completion

Completion is binary:

1. Run focused native, spec, machine, surface, property, trace, and package checks that apply. Record execution-derived traces with `rms trace run <implementation.yaml> --profile smoke --record`, then rerun without `--record` to compare them. Run every smoke property realization with `rms property run <implementation.yaml> --profile smoke`.
2. Run `rms gate --root .`; continue working if it exits nonzero or reports a failed check.
3. Commit the candidate.
4. Run `rms audit --root . --strict`; continue working if it exits nonzero.
5. Only then report completion, including the exact checks run.

Run the smallest checks that prove the changed promise:

- `rms validate --root .`
- `rms compose --root .`
- `rms spec check <module.yaml|implementation.yaml>` after semantic changes.
- `rms machine check <implementation.yaml>` when an implementation binding exists.
- `rms surface check <implementation.yaml> --strict` when runnable app, UI, CLI, browser, HTTP, batch, or executable entrypoints exist.
- `rms structure <implementation.yaml>` when an implementation binding exists and inner roles changed.
- `rms trace check <trace-bundle>`, `rms trace replay <trace-bundle>`, or `rms trace diagnose <trace-bundle>` when local transition evidence exists.
- `rms trace run <implementation.yaml> --profile smoke --record` after implementing transition behavior, followed by `rms trace run <implementation.yaml> --profile smoke` to prove the committed bundle matches fresh execution.
- `rms trace stitch <trace-bundle>...` when a scenario crosses module boundaries; diagnose the saved system trace to locate the first broken handoff.
- `rms property check <module.yaml|implementation.yaml>`, `rms property run <implementation.yaml>`, or `rms property replay <counterexample.yaml>` when laws, parsers, numeric bounds, reusable modules, or generated counterexamples are involved.
- `rms verify <implementation.yaml>` when the module has an implementation binding, or `rms verify <composite-module.yaml>` for composite rollups.
- `rms package <module.yaml>` when a module is intended for reuse outside its current owner; it records concrete package proof. Use `rms verify-package <package-dir>` for an independent recheck.
- `rms gate --root .` when reviewing a working-tree change.
- `rms audit --root . --strict` before claiming production-ready RMS software.

Strict audit requires a git source revision. Commit the production candidate before treating strict audit as release evidence. Strict audit executes declared smoke proof commands against committed code, compares regenerated traces and reusable packages, runs each property realization independently, and fails if a proof command mutates production files.

For stateful or workflow behavior, include transition records, golden timeline tests, replay bundles, and first-bad-transition diagnostics when they apply.

Do not declare an implemented module done while `rms validate --root .` reports `semantic.contract-scaffold-active`, `evidence.placeholder`, `evidence.bootstrap-active`, `evidence.source-unpinned`, or `evidence.semantic-shape-only` for that module. Replace scaffold contracts through `contracts.set` and replace scaffold evidence with concrete law, contract, boundary, scenario, trace, runtime, recovery, or reconciliation evidence. Evidence must not describe its source as a current filesystem snapshot or a repository without a Git revision; strict audit resolves the committed candidate revision.

Report remaining manual obligations explicitly, especially compatibility review, missing evidence, undeclared effects, or partial conformance.

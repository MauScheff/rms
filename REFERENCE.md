# RMS Reference

This is the comprehensive operational index for Reliable Modular Systems. It is intended for agents, maintainers, reviewers, and readers who need exact command behavior, classifications, artifacts, authority boundaries, and proof rules after the introductory [README](README.md).

This document is derived guidance, not an independent source of system meaning. In an RMS project, canonical artifacts own the architecture and behavior. For the RMS standard itself, [SPEC.md](SPEC.md), [MANIFEST.md](MANIFEST.md), the schemas, and the active CLI contracts are normative.

## Authority and Reading Order

| Need | Authority or guide |
| --- | --- |
| One project's meaning and architecture | Its `system.yaml`, `context-map.yaml`, `module.yaml`, contracts, applied revisions, implementation bindings, and active evidence |
| RMS normative semantics | [SPEC.md](SPEC.md) and [MANIFEST.md](MANIFEST.md) |
| Exact installed CLI syntax | Live `rms <command> --help` output |
| JSON behavior | Active contracts in `tooling/rust/rms/contracts/` and the CLI serializer; [TOOLING.md](TOOLING.md) is derived guidance |
| Conceptual explanation | [EXPLAINED.md](EXPLAINED.md) |
| First complete workflow | [QUICKSTART.md](QUICKSTART.md) |
| First replayable hunt and visual dogfood exercise | [FIRST_BUG_HUNT.md](FIRST_BUG_HUNT.md) |
| Production completion policy | [PRODUCTION.md](PRODUCTION.md) |
| Agent workflow | Project `AGENTS.md`, selected project skills, and rendered RMS context |
| Maintainer release process | [RELEASE.md](RELEASE.md) |

Reports, explanations, plans, prompts, graphs, packages, and command logs are derived evidence. They may cite canonical meaning but do not create or override it.

## Public Command Doorway

```text
rms init [OPTIONS] --name <NAME> --purpose <PURPOSE> [PATH]
rms next "<exact user task>" [--intent-json JSON | --intent-yaml YAML | --intent-file PATH | --ai [--refresh-intent]] [--root PATH] [--module MODULE] [--json] [--details]
rms explain ["<question>"] [--root PATH] [--module MODULE] [--json] [--details]
rms check [--environment | --changes | --committed | --all] [--root PATH] [--json] [--details]
rms view [OPTIONS]
rms help --all
```

`rms --help` shows the five primary commands plus the `help` meta-command. Specialist commands remain directly callable but appear only in the grouped `rms help --all` catalog.

### Human response grammar

`next`, `explain`, and `check` project rich internal evidence into a small default answer:

```text
Outcome or answer
Why
Next
Done when
```

The default includes one answer, at most three reasons, the immediate action, warnings when present, and completion conditions. `--details` adds the complete relevant evidence without changing the command's meaning.

### Agent JSON envelope

JSON reports from `next`, `explain`, and `check` use `rms.surface/v2`. `init` and `view` do not implement this report contract.

There is no separate JSON Schema for `rms.surface/v2`; the active contracts and serializer define it.

```text
schema
command
result
summary
reasons
warnings
next_action
done_when
details_available
```

Command-specific fields are additive:

| Command | Additional fields |
| --- | --- |
| `next` | `lane`, `confidence`, `owner`, ordered `steps` |
| `explain` | `answer`, canonical `evidence` paths |
| `check` | `mode`, constituent `components` |

`next_action` is one typed action or `null`. A `next` owner is `{status, module?, path?}`, where status is `selected`, `ambiguous`, `none`, `invalid`, or `not-required`; `steps` is the ordered, flattened action sequence. A `check` result is `pass`, `review-required`, or `fail`, and each component is `{id, result, summary}`.

`details_available` is currently always `true`. `--details --json` adds a `details` object under the same envelope; it never restores an older unversioned report.

### Machine probes

`rms probe` is the fast, diagnostic path for poking a real state machine without promoting the run to verification evidence:

```text
rms probe [IMPLEMENTATION] --describe
rms probe [IMPLEMENTATION] --input <JSON>... [--state <JSON>]
rms probe [IMPLEMENTATION] --file <PATH|->
rms probe --file <ASSEMBLY> --describe
rms probe --file <ASSEMBLY> [--explore] [--max-steps N] [--max-schedules N] [--max-states N]
rms probe --replay <COUNTEREXAMPLE>
```

It resolves the nearest `implementation.yaml`, or the only supported implementation beneath the RMS root. Ambiguity is an error with candidate paths. Rust, Swift, JavaScript, and Python probe adapters accept `rms/machine-probe/v0.1`; v0.2 additionally batches independent `{state,input}` evaluations. Both call the exact declared transition-record function and never invoke the driver or an effect executor.

Inline probes may assert `--expect-final-state` and `--expect-final-case`. Scenario files may assert per-step cases and outputs plus whole-run state and case paths, with recursive object-subset matching and exact ordered array/scalar matching. Normal runs write nothing; `--out` explicitly preserves the validated trace.

An `rms/probe-assembly/v0.1`, `v0.2`, or `v0.3` file composes any bounded set of instances through canonical dependency bridges and protocol bindings. Its scheduler is deterministic and virtual: exploration branches only across causally valid same-time deliveries, declared substitute outcomes, and explicitly enabled delay, duplicate, drop, or timeout faults. Checks run after microsteps and at quiescence or bounded deadlines. Passing exhaustive exploration means the bounded reachable space was exhausted; reaching any bound is `inconclusive`, never `pass`. v0.3 adds closed typed state expressions over RFC 6901 projections from the current states of named instances. Missing paths or runtime type mismatches invalidate the run; a false expression is a replayable check failure with structured observed facts.

Version 0.2 may declare campaign closure under `coverage`. `required_modules` names every adopted decision participant the requested campaign depends on. Each `fault_families[]` item names its semantic owner and one exact generator: `stimulus` by id, `substitute` by id, or `route-fault` by route plus `delay|duplicate|drop|timeout`. `rms hunt --assembly ... --dry-run` reports `unsupported` when an owner is not adopted, an adopted participant is absent from the assembly, or the declared generator is missing. This is a planning truthfulness check; it does not adopt missing modules or invent product laws.

v0.2 optionally derives a small workload from public machine-input examples:

```yaml
workload:
  source: public-input-examples
  budget_per_action: 3
```

Only public command bindings with exact, schema-valid probe examples are eligible. Workload injections coexist with stimuli, schedules, substitutes, and faults; the normalized input is stored in every decision so replay never regenerates or guesses it.

The committed `examples/probes/public-rust-workload-failures.yaml` provides a small intentionally failing guided-hunt path for learning and dogfood. Run it with `rms hunt --root . --assembly examples/probes/public-rust-workload-failures.yaml --budget 30s`. It is demonstration evidence, not a production claim.

A failure writes one minimized `rms/probe-counterexample/v0.1` only when `--out` is supplied. Human replay output leads with the result, failed check, first bad transition, source drift, exit meaning, and a copyable `--json` command for the full trace. Replay exits `0` when resolved, `1` when the failure reproduced, and `2` when invalid or no longer executable. Assemblies remain ephemeral diagnostics until canonical verification references them; referenced assemblies must exhaust successfully, and referenced counterexamples must replay as resolved.

### Typed actions

| Action kind | Fields |
| --- | --- |
| Command | `kind`, `phase`, `program`, `args`, safely escaped `display`, `authorization: none` |
| Manual | `kind`, `phase`, `instruction`, `authorization` |

Phases are `inspect`, `declare`, `implement`, `verify`, and `complete`; authorization is exactly `none` or `host-required`. Execute command actions by passing `args` directly to `program`. Never parse or execute the human-oriented `display` string. Candidate commits are always manual actions with `authorization: host-required`; RMS does not emit an executable Git commit command.

## Command Semantics

| Command | Required input and defaults | Execution and exit behavior |
| --- | --- | --- |
| `init` | `--name`, `--purpose`; path defaults to `.`, version to `0.1.0`; without `--context`, creates one context named after `--name` | The target directory may be created first, but managed artifacts wait for collision preflight; success `0`, operational failure `1`, syntax error `2` |
| `next` | Nonblank intent plus `rms/intent-model/v0.1` or explicit `--ai`; root defaults to `.`, module is optional | Read-only; validates typed facts and routes structured subjects; every constructed report `0`, impossible construction `1`, syntax error `2` |
| `explain` | Question and module are optional; root defaults to `.` | Read-only; every constructed report `0`, impossible construction `1`, syntax error `2` |
| `check` | No required argument; root defaults to `.`, mode defaults to project | May execute delegated proof commands but does not mutate canonical semantics; pass `0`, otherwise `1`, syntax error `2` |
| `view` | No required argument; root defaults to `.`, port to `7337` | Starts a long-running loopback server and may open a browser; startup failure `1`, syntax error `2` |
| `help` | `--all` is optional | Reads compiled command metadata only; success `0`, inconsistent compiled metadata `1`, malformed request `2` |

### `init`

`init` creates a fresh RMS system. `--adopt` integrates RMS into an existing repository while preserving compatible project-owned documents and changing only RMS-managed content.

```text
init → authorized bootstrap commit → design → recommended scaffold
```

Initialization may create the target directory before preflight, but it performs all collision checks before writing managed artifacts. A standalone target becomes a Git worktree; a target already inside a worktree continues to use that worktree. Successful initialization ends in:

```text
bootstrap prepared; provenance baseline pending authorized commit
```

The command prepares provenance but does not grant commit authority or production readiness.

Fresh repositories record `workspace.coverage: complete`; `init --adopt` records `progressive`. Inspect or change the claim with:

```bash
rms adoption status --root .
rms adoption set --root . --coverage complete --dry-run
```

Progressive affected root checks select changed RMS owners and add reverse dependents only for consumer-visible changes. They report native and outside-coverage paths without claiming RMS certification. Explicit `--module <module.yaml>` remains a caller-owned override that certifies the target, contained children, and transitive declared module providers. Complete exhaustive coverage is rejected while production paths remain outside RMS ownership.

Projects can declare native boundaries without adopting them into RMS:

```yaml
workspace:
  coverage: progressive
  native_workflows:
    - id: native-client
      paths: [clients/native]
      consumes: [service-provider]
      proof:
        local: [native-test]
        release: [native-release]
        hardware: [native-hardware]
```

RMS reports these commands but does not execute them. Equal-specificity path matches are invalid. Native and outside-coverage paths remain uncertified, and outside coverage records a gap rather than selecting an invented owner.

### `next`

Typed intent contains no architecture recommendation:

```yaml
spec: rms/intent-model/v0.1
operation: design
change_scope: new-module
subjects: [client-account-access]
facts:
  domain_decisions: { disposition: required, basis: explicit, source_quote: "pure Swift library" }
  lifecycle: { disposition: absent, basis: inferred, rationale: "No ordered behavior was requested." }
  effects: { disposition: absent, basis: explicit, source_quote: "performs no IO" }
  runnable_surface: { disposition: absent, basis: explicit, source_quote: "no runnable interface" }
  reuse: { disposition: required, basis: explicit, source_quote: "reusable pure Swift library" }
responsibilities:
  - { id: account-access-decisions, kind: decision, summary: "Decide account-access outcomes." }
binding_preferences: [swift]
open_questions: []
```

Every fact is `required`, `absent`, or `unknown`. Explicit facts quote the task exactly; inferred facts provide rationale. Material unknowns require clarification. `architecture`, `topology`, module names, shapes, and scaffold recommendations are forbidden in the model.

Typed-intent failures use the `intent.*` diagnostics; contract publication uses `semantic.contract-kind-missing` and `semantic.capability-binding-missing`; adoption uses `adoption.module-closure-unresolved` and `adoption.unowned-production-path`. `unsplit_runnable_justification` is valid only for one module mixing workflow decisions, a runnable boundary, and effects; ordinary standalone modules receive `semantic.unsplit-runnable-justification-not-applicable`.

`next` builds a prospective prescription from a validated `rms/intent-model/v0.1`, repository shape, canonical artifacts, routing evidence, implementation bindings, and declared proof obligations. The agent or read-only provider extracts facts; RMS rejects topology fields and chooses structure deterministically.

Report results are:

| Result | Meaning |
| --- | --- |
| `intent-model-required` | Supply a typed model or explicit read-only `--ai` extraction; no architecture advice is emitted. |
| `clarification-required` | A material fact is unknown; RMS stops rather than guessing. |
| `ready` | Ownership and the immediate RMS lane are sufficiently resolved. |
| `bootstrap-required` | The root needs RMS initialization or adoption before semantic work. |
| `design-required` | A boundary or semantic shape must be designed before implementation. |
| `needs-owner` | Ownership remains ambiguous or no unique owner can be selected; RMS will not guess. |
| `blocked` | Readable but invalid canonical evidence supports a truthful blocked report. |
| `no-rms-change` | The intent is a repository operation rather than an RMS semantic change. |

Task lanes are:

| Lane | Scope |
| --- | --- |
| `read-only` | Inspection, explanation, diagnosis, or proof with no project mutation |
| `design` | Module boundaries, capability shape, or initial system structure |
| `semantic` | Meaning, laws, contracts, properties, effects, dependencies, authorities, or evidence obligations |
| `surface` | CLI, UI, HTTP, batch, app, or another runnable boundary |
| `semantic-plus-surface` | Public meaning and its runnable boundary change together |
| `implementation-candidate` | Existing declared role bodies or an explicitly identified native boundary can realize existing canonical meaning |
| `repository-operation` | Installation, skill/plugin synchronization, or Git status/fetch/commit/rebase/merge/push |
| `undetermined` | Observable evidence cannot yet select a truthful lane |

Classification confidence is `deterministic` after model validation. `operation` chooses the lane, structured `subjects` route ownership, and facts/responsibilities choose topology. Explicit adoption of an existing source or runtime boundary into a missing canonical owner normalizes to `design` with `new-module` scope. Existing modules named only as consumed evidence, dependencies, executors, or consumers cannot become that new owner. Exact canonical owner paths, explicit in-place existing-owner work, and `--module` overrides keep their existing-owner semantics. Other raw task words, including words inside negation, have no architectural authority.

An explicit request to implement a native adapter, handler, executor, envelope, or wire mapping for an identified existing contract normalizes to `implementation-candidate` when the request does not change canonical meaning. The same rule applies to a native repair that explicitly preserves the identified existing contract. Detailed acceptance criteria can restate the existing promise. The ready receipt grants no canonical mutation family, does not prescribe `spec plan`, and reports native or outside-coverage source honestly through progressive checks. If the task adds or changes a contract, law, input, output, effect, protocol, ordering rule, or evidence obligation, the lane remains `semantic`.

`no-rms-change` contains no design, specification, source-edit, gate, audit, or pending-candidate prescription, even when the readable root is uninitialized. It reports only the repository operation and its applicable authority boundary.

Owner selection is deterministic:

1. An explicit readable `--module` wins.
2. Otherwise prefer a direct root `module.yaml`.
3. Otherwise select the sole top-level module.
4. Otherwise select one unique positive match from structured semantic subjects.
5. Recurse through declared composites using route evidence and cycle protection.
6. Stop at `needs-owner` for ties, non-positive multi-candidate matches, or recursive ambiguity.

Prospective declared-language scoring retains the full count of distinct matching purpose, ownership, behavior, and public-surface terms. It does not saturate stronger evidence into an artificial tie.

A recursive ownership cycle makes the owner invalid and produces `blocked`, not `needs-owner`. Direct-root and sole-top-level selection do not require a positive task-language score.

Constructed reports exit `0`, including bootstrap, design, ambiguity, blocked, and no-change states. Impossible construction exits `1`; command-line syntax errors exit `2`.

An unreadable root or explicit module makes construction impossible and exits `1`; it is not a constructed `blocked` result. `next` never mutates files, invokes providers, runs its prescribed verification, or grants source-edit or Git authority.

### `explain`

`explain` answers from canonical evidence. With no question it gives a concise module overview. Focused questions can cover purpose, ownership, public behavior, dependencies, invariants, machine state, effects, profiles, roles, verification, compatibility, and change protocol.

| Result | Meaning |
| --- | --- |
| `overview` | No question was supplied. |
| `answered` | A supported focused question was answered from canonical evidence. |
| `insufficient-evidence` | No supported deterministic focus matched the question. |
| `blocked` | Invalid readable evidence supports a truthful diagnostic answer. |

Unsupported questions return `insufficient-evidence` with the best deterministic evidence-gathering action. Explicit module selection wins; inferred selection succeeds only when one module is unambiguous.

Constructed reports, including insufficient evidence, exit `0`. Impossible construction exits `1`; syntax errors exit `2`. The primary command never invokes a provider or introduces architecture not present in canonical artifacts.

### `check`

`check` delegates to existing deterministic policy engines:

| Invocation | Mode | Delegated work |
| --- | --- | --- |
| `rms check --root .` | `project` | Validation plus composition |
| `rms check --environment --root .` | `environment` | Repository, tool, guidance, configuration, effective provider/model, and detected skill diagnosis |
| `rms check --changes --root .` | `changes` | Affected worktree delta against `HEAD` with baseline-debt comparison |
| `rms check --committed --root .` | `committed` | Affected clean `HEAD` delta against its first parent with committed provenance |
| `rms check --all --root .` | `all` | Exhaustive strict release/CI audit of the complete discovered repository |

The mode flags are mutually exclusive. Exit `0` means every check selected by the mode passed. Any failed or review-required aggregate exits `1`; syntax errors exit `2`.

`check` does not recursively invoke the CLI, duplicate delegated policy, mutate canonical semantics, or convert a dirty candidate into committed evidence.

Affected output includes a content-bound deterministic selection receipt, exact RMS-owned closures and selection reasons, native workflow handoffs, outside-coverage paths, candidate regressions, unchanged baseline debt, and full or partial coverage status. Only new candidate regressions block an otherwise unchanged baseline finding. Use `--all` when the claim requires complete-repository certification, strict proof regeneration, or release provenance.

### `view`

`view` serves a loopback-only, read-only projection of the semantic system graph. It combines ownership, contracts, machines, semantic functions, effects, proof declarations, status, and canonical source paths.

Supported boundary methods are GET and HEAD. `--watch` refreshes changed canonical inputs; `--no-open` suppresses browser launch; `--port 0` selects an available loopback port. The viewer never becomes a semantic authority.

## Repository Classification

RMS diagnoses the requested root as one of:

| Kind | Observable shape |
| --- | --- |
| `system-root` | Root `system.yaml` or `context-map.yaml` exists, including a partial or invalid root. |
| `module-root` | Exactly one module begins directly at the root and no system classification applies. |
| `system-container` | The root contains one discoverable RMS system below it. |
| `multi-system-workspace` | The root contains multiple discoverable RMS systems. |
| `module-workspace` | The root contains RMS modules but no governing system manifest. |
| `uninitialized` | No canonical RMS system or module is present. |

For container and workspace shapes, absent root-level `system.yaml` or `context-map.yaml` is `not-applicable`, not missing. A genuine partial system root remains missing or invalid.

## Semantic Model

| Concept | Responsibility |
| --- | --- |
| System | Names the product boundary, contexts, public interfaces, workflows, and global compatibility policy. |
| Context map | Declares bounded contexts and their relationships. |
| Module | Owns coherent concepts, data, decisions, profiles, dependencies, effects, invariants, and public behavior. |
| Contract | States the externally consumable meaning of a command, query, event, capability, API, or schema. |
| Law or invariant | States behavior that must always remain true. |
| Semantic function | Names the exact pure or effectful operation that discharges declared meaning. |
| Machine | Classifies state, commands, observed events, effect results, transitions, replies, and rejections. |
| Effect protocol | Names an outside-world request, typed result, executor, authority, and lifecycle policy. |
| Runnable surface | Adapts external input and delegates to a declared boundary or machine path. |
| Evidence | Connects a declared promise to a concrete scenario, trace, property, boundary check, runtime check, or reconciliation proof. |

Canonical artifacts include:

- `system.yaml` and `context-map.yaml`;
- each `module.yaml` and public contract;
- `implementation.yaml` bindings;
- hash-sealed applied semantic, machine, and surface revisions;
- laws, contract scenarios, boundaries, traces, properties, fuzz targets, runtime checks, and reconciliation evidence.

Implementation code fills roles declared by the canonical model. Another module may be used only through its public facade or contract-shaped entrypoint, never through private representation, transition, parser, adapter, or executor roles.

## Declaration Gates

| Requested change | Required declaration before implementation |
| --- | --- |
| Meaning, law, contract, effect, dependency, authority, property, or evidence obligation | Semantic apply, dry-run first |
| State, classified input/output, or transition structure | Semantic apply or focused machine apply |
| CLI, UI, HTTP, batch, app, or executable boundary | Surface apply |
| Module boundary or topology | Typed design, then exactly the recommended `add-module` or `add-capability-tree` action; signed child names and shapes must survive unchanged into the scaffold |
| Publish or require a capability on an existing module | `spec apply` with contract `kind: capability`, direction, and matching behavior binding |
| Deferred implementation binding | Add a binding before machine or surface work |
| Existing declared role body only | Edit the role, then run focused proof |

Do not hand-edit canonical manifests, contracts, semantic functions, behavior bindings, machine declarations, surfaces, protocols, authorities, resources, or evidence declarations. Use `set` and `remove` operations to revise canonical meaning, dry-run the complete change first, then apply and check it. Applied revisions are sealed history.

If the CLI cannot express a required semantic change, report the RMS gap rather than bypassing the declaration gate.

A focused probe-capable machine change uses the same `rms/machine-change/v0.1` object for the command binding, initial state, probe protocol, and adapter role:

```yaml
spec: rms/machine-change/v0.1
commands:
  probe: swift test --filter MachineProbeTests/testProbeMachine
machine:
  mode: stateful-transition-machine
  initial_state: Idle
probe:
  protocol: rms/machine-probe/v0.2
  command: probe
  runner: Tests/SecureMediaSessionTests/MachineProbeTests.swift#testProbeMachine
  initial_state_function: Sources/SecureMediaSession/Representation.swift#SecureMediaSessionWorkflowState.initial
roles:
  add:
    - kind: probe_adapter
      path: Tests/SecureMediaSessionTests/MachineProbeTests.swift
```

Omit unchanged optional sections. Run `rms machine apply <implementation.yaml> --change-yaml '<change>' --dry-run --route-receipt <receipt>` before the real apply; use `rms spec apply` instead when laws, contracts, effects, properties, or evidence obligations change.

## Proof and Completion

Focused proof is selected from the promises affected by the change:

| Proof lane | Specialist commands |
| --- | --- |
| Canonical validity and composition | `rms validate`, `rms compose` |
| Semantic declarations | `rms spec check` |
| Machine structure | `rms machine check` |
| Runnable boundaries | `rms surface check --strict` |
| Implementation roles | `rms structure`, project-native checks |
| Broad semantic properties | `rms property check`, `rms property run`, counterexample replay |
| Unattended strong verification | `rms hunt --dry-run`, then a budgeted `rms hunt` from a clean commit |
| Lifecycle evidence | `rms probe`, `rms trace check`, `rms trace show`, producer execution |
| Bound implementation or composite | `rms verify` |
| Reusable module | `rms package`, `rms verify-package` |
| Compatibility | `rms check-compat` |

### Executable property loop

Choose the specialist command by scope:

| Scope | Command | Boundary |
| --- | --- | --- |
| Legacy implementation metadata | `binding migrate` | Selects strict analyzers from exact `path#symbol` references, including Python and shell functions in an executable binding. Produces no candidate when trust, call, or authority inference is ambiguous. Requires a matching ready route receipt in dry-run and write modes. |
| One implementation's call/effect closure | `structure` | Runs transitive purity and authority analysis. It does not execute effects. |
| One implementation's probe schemas | `property generate` | Writes a deterministic probe assembly. It does not replace raw-parser fuzzing. |
| One focused transition | `probe` | Ephemeral diagnostic execution through the real transition-record path. |
| System contract closure | `compose --root` | Read-only. It does not execute probes or write artifacts. |
| Symbolic system exploration | `compose --output` | Executes probe `describe` only, validates the generated assembly, and writes derived artifacts atomically. |
| Universal finite proof | `property search --goal violate --out` | Emits a proof certificate only when the declared finite model is exhausted without a violation. |
| Open-ended discovery | `hunt` | Produces bounded evidence and replayable findings, never a universal claim. |

RMS behavioral contracts and temporal guarantees share one compiled reference evaluator. A v0.2 contract retains caller-obligation semantics. A v0.3 contract separates external assumptions from boundary-validatable requirements and is total under satisfied assumptions: invalid input must receive a typed, empty-frame rejection. An external clause names one exact executable module property. Canonical selections use IDs such as `contract:start-checkout#accepted-case`.

```text
rms property check TARGET
rms property generate IMPLEMENTATION --out ASSEMBLY [--property ID] [--seed N] [--cases-per-input N]
rms property evaluate TARGET --trace TRACE [--property ID] [--out ANALYSIS]
rms property search TARGET --assembly ASSEMBLY --goal satisfy|violate [--property ID]
rms property analyze TARGET [--assembly ASSEMBLY]
rms property monitor TARGET --input TRACE|- [--property ID]
rms property replay ANALYSIS
rms check-compat OLD_CONTRACT NEW_CONTRACT
rms spec migrate-contract INPUT [--out OUTPUT]
rms binding migrate IMPLEMENTATION --to v0.2 --route-receipt RECEIPT [--dry-run]
rms compose --root ROOT [--output DIR] [--seed N] [--cases-per-input N] [--dry-run] [--force]
```

`property generate` derives deterministic schema-valid workloads from a machine-probe v0.2 `describe` response. Equal implementation, schema, generator version, seed, and case count produce equal workloads. Unsupported schema keywords return an explicit unsupported result and require a manual generator.

`compose` stays read-only without `--output`. Output mode executes only `describe` operations and atomically writes `composition.json` plus `probe-assembly.yaml`. `--dry-run` validates without writing; existing output requires `--force`. Missing or ambiguous providers, incompatible contracts, non-dual protocol endpoints, unauthorized effects, unresolved mappings, dependency cycles, and lifecycle-result bypass prevent generation.

An exhaustive `property search --goal violate --out ANALYSIS` that finds no violation writes `ANALYSIS.proof-certificate.json`. Composition reuses it only when subject, contract, implementation, source, tool, strategy, assumptions, and evidence digests match exactly. These generated artifacts are evidence projections, not semantic authority. The complete operational guide is [Functional Core and Composition](FUNCTIONAL_CORE.md).

`evaluate` reports satisfaction only for supplied invocation or transition records. For v0.3, false or missing assumptions are inconclusive without blame; invalid acceptance, missing typed rejection, and mutation during invalid rejection assign provider blame. Postconditions, frames, and invariants also assign provider blame. v0.2 requirements retain caller blame. Malformed or incomplete evidence assigns binding/evidence blame.

With an assembly, `analyze` checks satisfiability, validity, vacuity, implication, equivalence, redundancy, and conflict over the finite model. With a direct core contract, it emits deterministic SMT-LIB v2 and calls optional cvc5 for satisfiability, coverage, disjointness, and consistency obligations. SAT models are re-evaluated by RMS. Missing cvc5, timeout, `unknown`, and unsupported theories remain unresolved. System-wide conclusions require exhausted finite exploration or exactly discharged solver obligations; a reached search bound is always inconclusive.

Metric observations and bounds use exact quantities. RMS v1 supports time, information, ratio, and nominal transition/message/attempt/item units. Compatible units normalize exactly; cross-dimensional comparisons are invalid.

Finite machine-, protocol-, resource-, artifact-, or composition-scope proof realizations use `strategy: deterministic-exhaustive` with `exhaustive: true`, or a model checker. The explicit flag prevents a finite sample from being mistaken for an exhausted space.

New operations write `rms/property-analysis/v0.2`; readers continue accepting v0.1 evidence. Stateless calls use `rms/invocation-record/v0.1`; stateful behavior continues to use transition records. `rms/compatibility-analysis/v0.1` records same-version refinement and directs callers to migrate before cross-version comparison. Streaming monitors remain fail-open. `full` streams may satisfy or violate; `sampled` and `partial` streams prove observed violations only; `delayed` prefixes stay inconclusive until declared complete; `none` is unsupported. Legacy unspecified observability never turns absence of a violation into success.

### Proof-first bug hunt

Risk determines the strong lane: generated or exhaustive checks for pure and numeric decisions; finite exploration for machines and workflows; coverage fuzzing for untrusted boundaries; schedule and fault exploration for distributed behavior; static analysis or sanitizers for unsafe authority; mutation testing for reusable semantic oracles; and real-trace evaluation plus violation search for temporal promises. `rms check --changes` and `--committed` require the appropriate declaration or a focused `verification.hunt_exceptions` entry, but continue to run only smoke proof.

```text
rms hunt --root . --dry-run
rms hunt --root . --budget 8h [--seed NUMBER] [--jobs NUMBER]
rms hunt --root . --assembly examples/probes/public-rust-workload-failures.yaml --budget 30s
rms hunt --root . --resume latest
```

The hunt requires a clean commit and runs project tools in an isolated checkout. `--module` and `--assembly` accept paths relative to `--root`, the Git repository root, or the caller's current directory. `--assembly` selects one assembly directly and runs only its guided semantic-novelty lane. The hunt checkpoints under ignored `.rms/hunts/<run-id>/`, passes `RMS_HUNT_RUN_ID`, `RMS_HUNT_SEED`, `RMS_HUNT_BUDGET_SECONDS`, and `RMS_HUNT_OUTPUT` to nightly runners, and validates their `rms/hunt-lane-result/v0.1` output. Resume restores the recorded module or assembly, budget, worker count, seed, and output path; explicitly changing one is rejected as configuration drift, and resuming an already finalized report is read-only so its finish timestamp remains immutable. For each declared probe assembly it runs a seeded semantic-novelty policy that favors new check outcomes, transition cases, states, routes, and faults, continues after failures, and retains up to eight distinct replayable counterexamples. `search_frontier_exhausted` describes the guided scheduler only; `proof_model_exhausted` remains false because guided evidence never becomes finite proof. Retain a canonical `rms/hunt-report/v0.2` only with `--out`; `.json` outputs contain JSON, other paths contain YAML, and readers continue to accept v0.1. `rms --version` includes the build revision so same-package binaries remain distinguishable.

Strict verification and `rms release check` use a content-addressed identity over source/declarations, relevant tool identities, and seed. A same-identity active process holds an ignored `.rms/cache/verification` lock and a duplicate exits immediately instead of contending on Cargo/package locks. Successful exact proof executions and completed release phases are resumable only for the unchanged identity. A successful unfiltered Rust `cargo test` suite can satisfy a focused realization only when its output names that exact test as passed; failed, ignored, substring-only, non-Rust, and counterexample cases still execute independently. Progress includes current/total, runner, elapsed time, ETA class, and whether the child is active, waiting on a package lock, or silent with deadlock explicitly not yet proven.

Outcomes are `bugs-found`, `proof-gaps-found`, `clean-under-recorded-bounds`, `inconclusive`, `invalid`, or `unsupported`. Surviving mutants and weak coverage are proof gaps. v0.2 findings have stable semantic IDs, occurrence counts, their property/check and first bad transition when available, and the shortest retained replay. Only explicitly exhaustive strategies can support a finite proof; guided and completed fuzz budgets remain bounded evidence.

During active implementation, run only the narrowest deterministic regression or compile check that can falsify the current hypothesis. For a hardware or distributed failure, run the focused project-owned physical smoke as soon as applicable prerequisite safety proof passes and one identical signed artifact is ready for every target. If the smoke fails, collect correlated evidence from that attempt and return to the narrow loop. Do not rerun unchanged broad gates. Migrations, destructive changes, security-sensitive changes, and changes that cannot safely reach hardware require applicable prerequisite proof before deployment.

After the focused happy path passes, leave the narrow loop. Run full owning-module verification, affected native suites, the affected RMS check, every returned broader acceptance or hardware gate, and the committed audit. This order does not waive candidate or release gates. The consumer repository owns the exact signing, installation, device, topology, and evidence procedure.

The project completion order is:

```text
focused happy-path proof
→ owning-module verification and affected native suites
→ rms check --changes --root .
→ returned broader acceptance or hardware gates
→ authorized candidate commit
→ rms check --committed --root .
→ rms check --all --root . for exhaustive release or CI certification
```

Without commit authority, stop at:

```text
candidate prepared; strict audit pending authorized commit
```

Git commits are required evidence, not implied authority. A commit establishes provenance only when the user task and host policy authorize it. The affected committed check records local candidate provenance. Exhaustive strict `rms check --all` must run against the clean committed candidate before complete RMS production readiness is claimed.

## Agents, Skills, and Providers

The CLI is sufficient; plugins and global skills are optional adapters. The normal agent sequence is:

1. Extract typed facts and run `rms next "<intent>" --root . --intent-yaml '<model>'` (or recorded read-only `--ai`).
2. Inspect the prescribed canonical context and declared paths.
3. Load the selected project skill.
4. Use `rms explain` when canonical meaning is unclear.
5. Follow specialist commands only when prescribed.
6. Complete through the two `rms check` proof boundaries.

Canonical skills live in `skills/`. Project-local Codex and Claude skills, embedded binary skills, plugin packages, and plugin caches are distributions. Diagnosis reports observable sources in deterministic origin/path order using `origin`, `scope`, `path`, `configured_state`, `digest`, `embedded_equivalence`, `runtime_activation`, `precedence`, `status`, and `remediation`.

Embedded equivalence is `canonical`, `equivalent`, `divergent`, `incomplete`, `unreadable`, or `unavailable`. Source status is `informational` or `review-required`.

Equivalent copies are informational. Divergent, incomplete, unreadable, or configuration-inconsistent copies are `review-required` and include project- or plugin-synchronization remediation.

Detection does not prove runtime activation in the current task. Runtime activation is `unknown`, and precedence is `host-defined`, because the CLI cannot inspect the host's injected skill catalog.

All five primary commands and help are provider-free. Provider-backed planning or explanation is explicit, specialist, and advisory until a canonical apply succeeds.

Before provider-backed `--ai` work, `rms check --environment --root .` verifies structured-output support and the effective Codex model against the installed CLI's bundled model catalog. `ai.codex.model` in project configuration takes precedence over the Codex user model; otherwise the provider default remains provider-owned. If readiness reports an unavailable or upgrade-gated model, update Codex and rerun the environment check, or explicitly pin `ai.codex.model` to a model supported by that installation. RMS never silently substitutes a model. If provider execution still fails, rerun the exact task after repairing the provider; do not infer an owner or synthesize typed intent as an automatic fallback.

Provider-backed semantic plans keep temporal validation strict. A property with observations or assumptions and no temporal expression is normalized to an explicit deferral that preserves its ordinary operation, oracle, and realizations. A repair may instead complete the property only with typed observations, identified `environment|search-preference` assumptions that contain closed expressions, a supported scope, and one closed temporal expression. Execution fields remain under realizations, never inside the temporal block.

RMS does not grant source-edit, provider, Git, release, deployment, or production authority.

## Specialist Command Groups

The exact catalog is generated by `rms help --all` from the same command definitions used for parsing. Its stable groups are:

- **Understand:** inspect, diagnose, context, route, atlas, probe.
- **Design and guide:** prompt, plan, design, review, refactor, implement, intent, evolve-contract, evidence.
- **Declare:** spec, machine, surface, add-module, add-binding, add-capability-tree, adoption.
- **Verify:** validate, impact, gate, trace, property, hunt, conformance, audit, check-compat, compose, verify, structure, package, verify-package.
- **Integrate:** run, dogfood, config, agent, release.

Specialist commands remain directly callable. Their absence from default help is presentation, not removal or a compatibility promise.

## Documentation Map

| Document | Scope |
| --- | --- |
| [README.md](README.md) | Project introduction and shortest successful path |
| [QUICKSTART.md](QUICKSTART.md) | Runnable onboarding and first complete change |
| [FIRST_BUG_HUNT.md](FIRST_BUG_HUNT.md) | Runnable finding-and-replay tutorial plus a visual Snake dogfood exercise |
| [EXPLAINED.md](EXPLAINED.md) | Conceptual model and motivation |
| [UNDERSTANDABILITY.md](UNDERSTANDABILITY.md) | Understandability laws, state-space review, and future self-hosting foundations |
| [FUNCTIONAL_CORE.md](FUNCTIONAL_CORE.md) | Tool selection for purity, schema generation, symbolic composition, and proof reuse |
| [PRODUCTION.md](PRODUCTION.md) | Production-pilot requirements and completion policy |
| [TOOLING.md](TOOLING.md) | Narrow-waist CLI and deterministic tooling model |
| [SPEC.md](SPEC.md) | Normative RMS semantic specification |
| [MANIFEST.md](MANIFEST.md) | Canonical manifest field reference |
| [GLOSSARY.md](GLOSSARY.md) | Stable RMS terminology |
| [DOGFOOD.md](DOGFOOD.md) | Self-hosted RMS walkthrough |
| [integrations/README.md](integrations/README.md) | Codex, Claude Code, and generic-agent adapters |
| [RELEASE.md](RELEASE.md) | Maintainer release proof and publication workflow |

## Version and Status

This repository is the RMS 0.1 Canonical Draft. The semantic core is frozen for pilot use. The Rust reference implementation is `0.1.0-rc.9`; the public presentation is intentionally narrow while the specialist engines remain available.

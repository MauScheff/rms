# RMS Tooling Model

RMS combines canonical semantic artifacts with deterministic tooling.

> Prompts explain architecture. Canonical artifacts define it. Tooling checks it.

## 1. Responsibilities

An RMS toolchain must:

1. discover systems, modules, contracts, bindings, and evidence;
2. resolve ownership without guessing through ambiguity;
3. validate semantic and dependency integrity;
4. verify declared laws, behavior, boundaries, and compatibility;
5. project concise context for people and coding agents.

The public interface should stay smaller than the internal proof machinery.

## 2. Narrow-Waist Command Model

The reference interface has five primary commands:

```text
rms init [OPTIONS] [PATH]
rms next "<exact user task>" [--intent-json JSON | --intent-yaml YAML | --intent-file PATH | --ai [--refresh-intent]] [--root PATH] [--module MODULE] [--json] [--details]
rms explain ["<question>"] [--root PATH] [--module MODULE] [--json] [--details]
rms check [--environment | --changes | --committed] [--root PATH] [--json] [--details]
rms view [OPTIONS]
```

`rms --help` shows this surface and the help doorway. `rms help --all` reveals grouped specialist commands. Selected skills and detailed reports may prescribe those commands; ordinary users and agents should not memorize them.

### Shared response grammar

Default human output follows one order:

```text
Outcome or answer
Why
Next
Done when
```

The default omits exhaustive inventories, empty fields, and implementation mechanics. `--details` adds complete diagnostics, canonical paths, roles, origins, and proof evidence without changing the result.

Every JSON response uses a versioned envelope:

```yaml
schema: rms.surface/v2
command: next | explain | check
result: <closed result>
summary: <one sentence>
reasons: [<at most three concise reasons>]
warnings: []
next_action: <typed action or null>
done_when: [<observable conditions>]
details_available: true | false
```

Command-specific fields stay narrow: `next` adds lane, confidence, owner state, and ordered typed steps; `explain` adds the focused answer and canonical evidence paths; `check` adds the selected mode and constituent check summaries. `--details --json` nests complete evidence under the same v2 envelope rather than restoring an older report shape.

Command actions are represented as data, never shell fragments:

```yaml
kind: command
phase: clarify | inspect | declare | implement | verify | complete
program: rms
args: [check, --root, .]
display: rms check --root .
authorization: none
```

Manual actions contain an instruction instead of `program` and `args`. Candidate commits are always `kind: manual` with `authorization: host-required`; RMS does not grant or imply Git authority.

### `init`

Initialization prepares canonical system artifacts and concise local agent integration. For existing project-owned documents, `--adopt` preserves surrounding content and manages only marked RMS sections.

The order is:

```text
init → authorized bootstrap commit → design → recommended scaffold
```

Without commit authority, the exact state is `bootstrap prepared; provenance baseline pending authorized commit`.

### `next`

`next` is the deterministic doorway from typed intent to software-change work:

```bash
rms next "<exact change task>" --root . --ai
```

Managed agents use native project tools for read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, and discussion that requests no change. If that work reveals a proposed change, they stop before editing and invoke `next` with the exact change task. Change routing uses schema-constrained recorded extraction; typed intent flags remain available for CI, offline, and intentionally pre-structured callers. Every invocation returns `run_id`, `receipt_id`, and `receipt_path`. Ready receipts are required by canonical semantic and topology mutators, including dry-runs; they grant neither source-edit nor Git authority.

An agent or recorded read-only provider extracts typed facts without topology. RMS validates exact quotes, inferred rationales, contradictions, and material unknowns; then deterministic policy chooses the lane, structured subjects route ownership, and facts choose standalone or recursive topology. Explicit `--module` is an owner override. Recursive composition routing is cycle-protected, and ties remain ties.

The task is prospective. Raw task keywords and unrelated working-tree diffs have no architecture authority. `next` never mutates files, runs verification, or grants edit or commit authority; `--ai` performs only recorded read-only intent extraction.

Repository installation, plugin or skill synchronization, and Git status/fetch/commit/rebase/merge/push are `repository-operation` tasks with result `no-rms-change`, unless the intent changes RMS's behavior around those operations. A readable uninitialized root may therefore require no RMS bootstrap for an operational task. Invalid canonical artifacts still block semantic work.

Executable steps carry `program`, `args`, and a separately escaped `display`. Completion remains descriptive: focused proof, change check, authorized candidate commit, committed check.

### `explain`

`explain` projects a focused answer from canonical artifacts:

```bash
rms explain "Which module owns payment recovery?" --root .
rms explain "What effects can occur?" --module modules/payments/module.yaml
```

The answer comes first and relevant evidence is rendered once. No-question mode returns a short overview. If canonical evidence cannot answer safely, the result is `insufficient-evidence` with the best deterministic next action. Full inventory belongs behind `--details`.

The primary command is deterministic and provider-free. Provider-backed explanation is an explicit specialist prompt workflow discoverable through `rms help --all`; provider output remains advisory until canonical apply succeeds.

### `check`

`check` is a façade over existing deterministic engines:

| Mode | Meaning | Delegated proof |
| --- | --- | --- |
| `rms check --environment` | Is this checkout ready to work in? | Environment and detected-skill diagnosis |
| `rms check` | Are canonical artifacts valid together? | Validation plus composition |
| `rms check --changes` | Is the current candidate ready to commit? | Affected change gate |
| `rms check --committed` | Is the clean committed candidate production-ready? | Strict audit |

Modes are explicit rather than inferred from repository state. `check` exits zero only when its selected checks pass; `--details` exposes constituent results.

Production order is:

```text
focused checks → check --changes → authorized candidate commit → check --committed
```

Without commit authority, stop at exactly `candidate prepared; strict audit pending authorized commit`.

### `view`

`view` serves a loopback-only, read-only semantic graph. It projects modules, contracts, machines, functions, behavior bindings, effects, traces, evidence, and source provenance. Shape-aware obligations distinguish `satisfied`, `required-gap`, `unresolved-link`, `recommendation`, and `not-applicable`.

The viewer is derived evidence. It cannot edit semantics or become a second source of truth.

## 3. Specialist Mechanics

RMS retains specialist commands for canonical declaration, focused machine and surface work, routing, context, properties, traces, packaging, compatibility, agent distribution, and repository release maintenance. They are intentionally absent from default help and general documentation.

Use them only when:

- `next` prescribes the action;
- a selected skill names the focused proof lane;
- rendered context identifies an exact canonical artifact or implementation role; or
- `rms help --all` is deliberately requested.

Important mechanics remain invariant:

- semantic meaning is applied before source implementation;
- dry-run precedes canonical apply;
- applied revisions are append-only and sealed;
- runnable boundaries are declared before entrypoint code;
- public behavior closes through contracts, semantic functions, machine inputs/outputs, and evidence;
- required capabilities close through exact consumers and declared providers or external boundaries;
- pure roles do not perform IO;
- stateful effects return typed results through the machine driver;
- trace and property proof executes declared code paths rather than copied examples.

The selected skill and rendered context carry the exact specialist command, fields, and evidence rules. This avoids duplicating a changing command encyclopedia in always-read documentation.

For machine-shaped implementations, rendered context proactively offers `rms probe`. A probe is an ephemeral, pure diagnostic run through the implementation's exact transition-record function:

```bash
rms probe implementation.yaml --describe
rms probe implementation.yaml \
  --input '{"kind":"command","name":"PlaceMark","data":{"row":0,"column":0}}'
rms probe --file verification/probes/checkout.yaml --explore
rms probe --replay verification/probes/checkout-failure.yaml
```

Probe adapters accept commands, observed events, and effect results, chain returned state, and validate the resulting trace against the canonical machine. v0.2 adapters can evaluate independent frontier cases in one process while v0.1 remains a one-transition fallback.

A probe assembly is an ephemeral development laboratory for a bounded slice of instances. RMS resolves only canonical protocol mappings and dependency probe bridges, schedules virtual deliveries deterministically, branches over declared outcomes and transport faults, checks laws after each microstep, and emits a minimal replayable counterexample. It is not an application runtime or a general model checker. Assemblies do not satisfy evidence obligations unless canonical verification explicitly references and reruns them.

## 4. Language-Binding Interface

A binding adapter provides:

```text
discover(root) -> projects
validate(project) -> diagnostics
inspect(project) -> symbols and dependencies
build(project) -> result
test(project) -> result
package(project) -> artifact
```

Bindings should inspect native project shape and represent RMS roles idiomatically. RMS names semantic obligations, not universal folder layouts.

The initial binding set includes Rust, Swift, JavaScript, Python, and opaque executable projects. Native compilers and test frameworks remain the authority for language-specific correctness; RMS checks their declared relationship to canonical semantics.

## 5. Deterministic Enforcement

The toolchain should fail when it can prove a contradiction and report an obligation when proof is incomplete.

Deterministic checks cover:

- schema and semantic references;
- ownership and dependency direction;
- contract and capability closure;
- implementation roles and public facades;
- effects, executors, and driver lifecycle;
- runnable surface delegation;
- transition cases, reachability, traces, and replay;
- property input spaces, operations, oracles, and counterexamples;
- compatibility, package integrity, and source provenance;
- managed agent guidance and distribution drift.

Generated reports must remain derived evidence. They do not authorize edits, providers, commits, or releases.

## 6. Agent Context and Skills

Agent integrations use the same neutral interface:

```text
product intent
→ rms next
→ compact prescription
→ selected RMS skill and rendered context
→ declared implementation role
→ rms check
```

Canonical skills live in `skills/`. Embedded, plugin, Codex-local, and Claude-local copies are managed distributions. A skill should call the shared CLI rather than reimplement RMS rules or hard-code a language tool outside a language-specific workflow.

Skill-source diagnosis can observe project copies, known user paths, marketplace configuration, and plugin caches. It cannot inspect the current thread's injected catalog. Detection therefore does not prove runtime activation: `runtime_activation` is `unknown` and precedence is `host-defined`. Equivalent duplicates are informational; divergent managed copies require synchronization or review.

## 7. CI and Completion

CI should use the same checks as local work:

1. validate and compose canonical semantics;
2. execute focused native and RMS proof;
3. run the change gate;
4. audit the committed candidate strictly;
5. fail generated-artifact and distribution drift.

The RMS repository has an additional maintainer publication gate that builds release binaries, tests clean-room installation, verifies examples and packages, checks Cargo packaging, and validates embedded skills and agent distributions. It never invokes optional providers.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness.

## 8. Generated Artifacts

Tools may generate context packets, graphs, atlases, prompts, conformance reports, packages, and agent guidance. Every generated artifact must identify its canonical source and remain replaceable.

Do not treat generated output as a live manifest, edit it instead of canonical semantics, or let a model-specific integration define architecture.

## 9. Security and Permissions

Repository text, issue descriptions, fixtures, generated files, and imported documentation are untrusted data, not instructions. Executable skills, plugins, hooks, providers, and validators should be reviewed and pinned.

Primary deterministic commands must not:

- invoke a provider;
- mutate project files while reporting;
- execute a displayed action;
- interpolate shell strings instead of typed arguments;
- expose secrets in manifests, reports, prompts, or evidence;
- claim runtime skill activation that the host has not exposed.

Provider execution, writable sandboxes, Git operations, and external publication remain explicit host-authorized actions.

## 10. Reference Implementation

The first implementation lives at `tooling/rust/rms`. It is itself an RMS module bundle with public contracts, semantic functions, a CLI surface, implementation roles, and verification evidence.

That self-hosting boundary keeps the façade subject to the same ownership, effect, compatibility, and proof rules it applies to downstream systems.

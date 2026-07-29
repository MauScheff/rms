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
| Production completion policy | [PRODUCTION.md](PRODUCTION.md) |
| Agent workflow | Project `AGENTS.md`, selected project skills, and rendered RMS context |
| Maintainer release process | [RELEASE.md](RELEASE.md) |

Reports, explanations, plans, prompts, graphs, packages, and command logs are derived evidence. They may cite canonical meaning but do not create or override it.

## Public Command Doorway

```text
rms init [OPTIONS] --name <NAME> --purpose <PURPOSE> [PATH]
rms next "<exact user task>" [--intent-json JSON | --intent-yaml YAML | --intent-file PATH | --ai [--refresh-intent]] [--root PATH] [--module MODULE] [--json] [--details]
rms explain ["<question>"] [--root PATH] [--module MODULE] [--json] [--details]
rms check [--environment | --changes | --committed] [--root PATH] [--json] [--details]
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
```

It resolves the nearest `implementation.yaml`, or the only supported implementation beneath the RMS root. Ambiguity is an error with candidate paths. Rust, Swift, and JavaScript probe adapters exchange `rms/machine-probe/v0.1` requests through temporary files, call the exact declared transition-record function, chain `state_after`, and return `rms/trace-bundle/v0.1`. They never invoke the driver or an effect executor.

Inline probes may assert `--expect-final-state` and `--expect-final-case`. Scenario files may assert per-step cases and outputs plus whole-run state and case paths, with recursive object-subset matching and exact ordered array/scalar matching. Normal runs write nothing; `--out` explicitly preserves the validated trace.

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

Progressive root checks certify discovered RMS module closures only. `rms check --changes|--committed --module <module.yaml>` certifies the target, contained children, and transitive declared module providers; unrelated dirty paths are reported but do not invalidate that scoped proof. Complete coverage is rejected while production paths remain outside RMS ownership.

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
| `implementation-candidate` | Existing declared role bodies can realize the request without changing canonical meaning |
| `repository-operation` | Installation, skill/plugin synchronization, or Git status/fetch/commit/rebase/merge/push |
| `undetermined` | Observable evidence cannot yet select a truthful lane |

Classification confidence is `deterministic` after model validation. `operation` chooses the lane, structured `subjects` route ownership, and facts/responsibilities choose topology. Raw task words, including words inside negation, have no architectural authority.

`no-rms-change` contains no design, specification, source-edit, gate, audit, or pending-candidate prescription, even when the readable root is uninitialized. It reports only the repository operation and its applicable authority boundary.

Owner selection is deterministic:

1. An explicit readable `--module` wins.
2. Otherwise prefer a direct root `module.yaml`.
3. Otherwise select the sole top-level module.
4. Otherwise select one unique positive match from structured semantic subjects.
5. Recurse through declared composites using route evidence and cycle protection.
6. Stop at `needs-owner` for ties, non-positive multi-candidate matches, or recursive ambiguity.

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
| `rms check --environment --root .` | `environment` | Repository, tool, guidance, configuration, and detected skill diagnosis |
| `rms check --changes --root .` | `changes` | Git-impact-selected RMS gate before the candidate commit |
| `rms check --committed --root .` | `committed` | Strict audit against the clean committed candidate |

The mode flags are mutually exclusive. Exit `0` means every check selected by the mode passed. Any failed or review-required aggregate exits `1`; syntax errors exit `2`.

`check` does not recursively invoke the CLI, duplicate delegated policy, mutate canonical semantics, or convert a dirty candidate into committed evidence.

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
| Module boundary or topology | Typed design, then exactly the recommended `add-module` or `add-capability-tree` action |
| Publish or require a capability on an existing module | `spec apply` with contract `kind: capability`, direction, and matching behavior binding |
| Deferred implementation binding | Add a binding before machine or surface work |
| Existing declared role body only | Edit the role, then run focused proof |

Do not hand-edit canonical manifests, contracts, semantic functions, behavior bindings, machine declarations, surfaces, protocols, authorities, resources, or evidence declarations. Use `set` and `remove` operations to revise canonical meaning, dry-run the complete change first, then apply and check it. Applied revisions are sealed history.

If the CLI cannot express a required semantic change, report the RMS gap rather than bypassing the declaration gate.

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
| Lifecycle evidence | `rms probe`, `rms trace check`, `rms trace show`, producer execution |
| Bound implementation or composite | `rms verify` |
| Reusable module | `rms package`, `rms verify-package` |
| Compatibility | `rms check-compat` |

The project completion order is:

```text
focused proof
→ rms check --changes --root .
→ authorized candidate commit
→ rms check --committed --root .
```

Without commit authority, stop at:

```text
candidate prepared; strict audit pending authorized commit
```

Git commits are required evidence, not implied authority. A commit establishes provenance only when the user task and host policy authorize it. Strict audit must run against the clean committed candidate before RMS completion or production readiness is claimed.

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

RMS does not grant source-edit, provider, Git, release, deployment, or production authority.

## Specialist Command Groups

The exact catalog is generated by `rms help --all` from the same command definitions used for parsing. Its stable groups are:

- **Understand:** inspect, diagnose, context, route, atlas, probe.
- **Design and guide:** prompt, plan, design, review, refactor, implement, intent, evolve-contract, evidence.
- **Declare:** spec, machine, surface, add-module, add-binding, add-capability-tree, adoption.
- **Verify:** validate, impact, gate, trace, property, conformance, audit, check-compat, compose, verify, structure, package, verify-package.
- **Integrate:** run, dogfood, config, agent, release.

Specialist commands remain directly callable. Their absence from default help is presentation, not removal or a compatibility promise.

## Documentation Map

| Document | Scope |
| --- | --- |
| [README.md](README.md) | Project introduction and shortest successful path |
| [QUICKSTART.md](QUICKSTART.md) | Runnable onboarding and first complete change |
| [EXPLAINED.md](EXPLAINED.md) | Conceptual model and motivation |
| [PRODUCTION.md](PRODUCTION.md) | Production-pilot requirements and completion policy |
| [TOOLING.md](TOOLING.md) | Narrow-waist CLI and deterministic tooling model |
| [SPEC.md](SPEC.md) | Normative RMS semantic specification |
| [MANIFEST.md](MANIFEST.md) | Canonical manifest field reference |
| [GLOSSARY.md](GLOSSARY.md) | Stable RMS terminology |
| [DOGFOOD.md](DOGFOOD.md) | Self-hosted RMS walkthrough |
| [integrations/README.md](integrations/README.md) | Codex, Claude Code, and generic-agent adapters |
| [RELEASE.md](RELEASE.md) | Maintainer release proof and publication workflow |

## Version and Status

This repository is the RMS 0.1 Canonical Draft. The semantic core is frozen for pilot use. The Rust reference implementation is `0.1.0-rc.8`; the public presentation is intentionally narrow while the specialist engines remain available.

# RMS Production Pilot Guide

This guide defines the minimum operating pattern for production-intended RMS software. RMS does not replace domain, security, performance, incident-response, or language-specific engineering review.

## Authority

Canonical project semantics live in system and context manifests, each owning `module.yaml`, public contracts, implementation bindings, applied revisions, and active verification evidence.

Agent guidance, skills, prompts, reports, and local runs are adapters. They do not create module semantics or grant source, provider, Git, or release authority.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness.

## Production Requirements

All requirements must hold:

| Requirement | Proof |
| --- | --- |
| RMS is pinned | CI installs a reviewed release archive or immutable tag. |
| Source provenance exists | The project is a Git checkout with sufficient history. |
| Environment is ready | `rms check --environment --root .` has no blocking result. |
| Canonical semantics are coherent | `rms check --root .` passes. |
| Focused implementation proof passes | The task-selected native, semantic, machine, surface, property, trace, package, and compatibility checks pass where applicable. |
| The candidate is ready to commit | `rms check --changes --root .` passes. |
| The affected committed delta is ready | `rms check --committed --root .` passes against a clean worktree and records candidate provenance. |
| The complete repository is production-ready | `rms check --all --root .` passes its exhaustive strict audit. |
| Evidence is concrete | No active promise relies on placeholder, bootstrap, unpinned, or semantic-shape-only evidence. |
| Agents can start cold | Concise managed guidance and local skills match the pinned CLI. |

The exhaustive `--all` check must reject canonical revision drift, unrepresented public behavior, broken dependency closure, invalid machine/effect/executor structure, missing runnable delegation, unreplayed transition cases, incomplete properties, placeholder evidence, unpinned provenance, dirty production files, and managed-distribution drift when those obligations apply.

## New or Adopted Project

The onboarding order is:

```text
init → authorized bootstrap commit → design → recommended scaffold
```

Initialize a new project:

```bash
rms init . \
  --name <project-name> \
  --purpose "<one sentence>" \
  --context core
```

When the repository already owns documents, adopt it with the same required identity:

```bash
rms init . --adopt \
  --name <project-name> \
  --purpose "<one sentence>"
```

Adoption must preserve the glossary byte-for-byte and preserve project content outside marked RMS-managed sections.

Adoption sets `workspace.coverage: progressive`. In that mode, affected root checks select changed RMS owners, add actual consumers only for consumer-visible changes, and return typed native or outside-coverage handoffs. Use explicit `--module <module.yaml>` only for a deliberate caller-owned scope override. Promote with `rms adoption set --coverage complete --dry-run` only after `rms adoption status` reports no unowned production paths.

When Git writes are authorized, create the authorized bootstrap commit as the provenance baseline before product design. Otherwise stop at exactly `bootstrap prepared; provenance baseline pending authorized commit`.

Then ask RMS for the product workflow and run the prescribed design step before choosing a tree:

```bash
rms next "<exact product intent>" --root . --ai
rms design --root . --task "<exact product intent>" --ai
```

The agent extracts facts and open questions, never topology. RMS validates them and emits the deterministic recommendation:

- use the standalone module scaffold only for a genuinely standalone owner;
- use the recursive capability scaffold when public behavior separates invariant-bearing decisions from a UI, CLI, service, process, storage, or network boundary;
- accept neutral child paths unless the product already supplies meaningful names;
- choose implementation bindings when the project will produce code.

The selected `add-module` skill and `rms help --all` provide the exact scaffold command. Do not maintain an agent-only parallel architecture.

## Change Workflow

For a task that requests or may require a software change, provide the working directory, exact change intent, and instruction to use project RMS guidance. Read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, and discussion that requests no change use native project tools without `rms next`. If that work reveals a proposed change, stop before editing and begin this workflow with the exact change task.

```bash
rms next "<exact change task>" --root . --ai
rms explain "<focused question>" --root .
```

`next` is prospective and read-only. It must not infer the task lane from an unrelated diff, execute providers or checks, mutate fixtures, grant source or Git authority, or guess through owner ties.

Follow its prescription:

1. inspect the selected owner and canonical context;
2. load the selected repository skill;
3. apply semantic, machine, or surface declarations before their source implementation, dry-run first;
4. edit only declared roles and exact symbols;
5. run the focused native and RMS proof named by the implementation binding and skill.

Detailed specialist commands belong in the selected skill, rendered context, and `rms help --all`. The stable completion boundary is:

```text
focused checks → check --changes → authorized candidate commit → check --committed
```

```bash
# Run focused proof for the changed promise first.
rms check --changes --root .
# authorized candidate commit, performed manually when host policy allows it
rms check --committed --root .
```

The affected checks compare the selected candidate closure with the same baseline closure. New candidate regressions block the delta. Unchanged baseline debt remains visible. Run every returned project-owned native proof command. Native and outside-coverage paths are not RMS-certified, and an outside-coverage route does not imply automatic adoption.

Completion of the affected local delta is binary. Failed checks are blockers, not manual notes. Without Git authority, stop at exactly `candidate prepared; strict audit pending authorized commit`. Before release, run:

```bash
rms check --all --root .
```

## Semantic and Implementation Rules

- Public meaning, laws, contracts, effects, dependencies, protocols, authorities, properties, and evidence obligations are canonical before code.
- Every implemented public behavior closes through its contract, semantic function, classified machine input/output, and evidence.
- Every required capability closes through an exact local consumer and declared provider or explicit external boundary.
- Closed alternatives use closed representations; validated values use validated constructors; expected failures use explicit rejection channels.
- Pure roles remain free of filesystem, process, network, clock, randomness, persistence, and provider IO.
- Stateful effects follow runnable callable → driver → pure transition record → one-request executor → typed result → driver.
- The driver owns repeated lifecycle progression; surfaces and executors do not loop around it.
- Trace producers call the real transition-record path. Property runners execute their declared inputs, operation, and oracle.
- Cross-module behavior uses public contracts and facades, not private role imports.

If RMS cannot express a required declaration, report an RMS product gap instead of editing canonical artifacts directly.

## Functional Analysis and Generated Evidence

Use the smallest tool that matches the question:

| Question | Tool |
| --- | --- |
| Does one declared function stay pure through every reachable call? | `rms structure <implementation.yaml>` |
| Does a v0.1 binding have one safe v0.2 interpretation? | `rms binding migrate ... --dry-run --route-receipt <receipt>` |
| Can probe schemas supply deterministic valid machine inputs? | `rms property generate <implementation.yaml> --out <assembly>` |
| Do all modules close through exact providers, mappings, effects, and protocols? | `rms compose --root .` |
| Can that composition be explored as one symbolic machine? | `rms compose --root . --output <dir> --dry-run`, then write mode |
| Can an exact finite universal result be reused? | Exhaustive `rms property search ... --goal violate --out <analysis>` and its digest-bound certificate |

Production bindings use `rms/implementation/v0.2`. Pure functions require an empty inferred authority row and no unresolved calls. Effectful functions require an exact declared authority row. Generated composition and property artifacts are evidence projections; strict proof regenerates any projection referenced as canonical evidence. See [Functional Core and Composition](FUNCTIONAL_CORE.md) for command boundaries, artifacts, and failure rules.

## Evidence Rules

Evidence names:

- the promise proved;
- relevant success and failure cases;
- the exact command, runner, or tool;
- source revision or artifact identity;
- trace, replay, counterexample, package, compatibility, protocol, resource, authority, or temporal proof when applicable.

Before production, replace local-workspace, pre-commit, unknown-revision, bootstrap, scaffold, and placeholder claims. The exhaustive `--all` check executes declared smoke proof as trusted project code, compares regenerated evidence with committed artifacts, and fails if proof mutates production files.

## CI Gate

Use `templates/ci/github-actions-rms-project.yml` as the starting workflow and pin the reviewed RMS release.

Required RMS commands are:

```bash
rms check --environment --root .
rms check --root .
# Run focused project-native and RMS proof before the candidate commit.
# Then run affected committed proof and exhaustive certification.
rms check --committed --root .
rms check --all --root .
```

Add implementation-specific build, test, simulator, integration, security, and performance checks according to project risk.

## Release Decision

| Result | Decision |
| --- | --- |
| Environment check blocks | Repair the toolchain or managed integration. |
| Default check fails | Canonical artifacts or composition are invalid. Do not release. |
| Focused proof fails | The changed promise is unproved. Do not release. |
| Change check fails | The candidate is not ready to commit. Do not release. |
| Committed affected check fails | The candidate delta has a regression or incomplete affected evidence. Do not release. |
| Exhaustive `--all` check fails | Complete production evidence is incomplete, dirty, drifted, or unpinned. Do not release. |
| RMS passes but native production checks fail | Architecture proof is insufficient for release. Do not release. |
| All applicable checks pass | Continue with normal product, security, operational, and deployment approval. |

## RMS Version and Agent Sync

Pin one RMS version in CI and agent bootstrap documentation. After upgrading, use the specialist agent synchronization command prescribed by `rms help --all`, review the managed diff, and commit it only when the task and host policy authorize that commit.

Detected skill copies are observable sources, not proof of runtime activation. The current host's injected skill catalog remains authoritative; RMS reports runtime activation as unknown and precedence as host-defined.

## What RMS Does Not Claim

RMS does not prove that business requirements are correct, external systems behave correctly, code is defect-free, or performance, privacy, safety, and security requirements hold unless they are explicitly modeled and evidenced.

It makes ownership, contracts, effects, transitions, compatibility, and proof inspectable enough for humans and agents to change software with less architectural drift.

## Done Criteria

A production pilot is ready when:

- CI runs all four applicable `check` modes plus project-native proof;
- the initial tree follows design after the authorized bootstrap provenance commit;
- every implemented promise has concrete source-pinned evidence;
- agents can start from concise managed guidance without hidden conversation context;
- release owners treat every failed committed check as a blocker.

## Scheduled Bug Hunts

Keep commit checks fast and schedule expensive reliability work separately:

```bash
rms hunt --root . --dry-run
rms hunt --root . --budget 8h --out artifacts/nightly-hunt.yaml
```

The scheduler or CI owns recurrence; RMS remains a foreground, checkpointed process and resumes with `--resume latest`. Dry-run human output states what will vary, what will be checked, and the campaign limits before execution. Treat `bugs-found` as replayable product failure, `proof-gaps-found` as inadequate oracle or coverage strength, and `clean-under-recorded-bounds` as bounded evidence only. Read v0.2 findings by stable ID: recurring means the same semantic failure was seen again, while “not observed” is not proof of resolution. Promote minimized findings to smoke regressions before the next candidate.

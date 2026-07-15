# RMS Production Pilot Guide

This guide defines the minimum operating pattern for using RMS on production-intended software.

RMS is production-pilot ready when the project uses canonical RMS artifacts as the architecture source of truth, all implemented modules have concrete evidence, and CI gates changes with deterministic RMS checks. It is not a substitute for domain review, security review, load testing, incident response, or language-specific engineering discipline.

## Authority

Canonical project semantics live in:

- `system.yaml`
- `context-map.yaml`
- `GLOSSARY.md`
- each owning `module.yaml`
- public contracts under `contracts/`
- implementation bindings under `implementation.yaml`
- active evidence under `verification/`

Agent instructions, plugin skills, generated prompts, and local run records are adapters. They help agents work inside RMS, but they do not create module semantics.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness.

## Production Pilot Requirements

A project is ready for production pilot use when all requirements hold:

| Requirement | Proof |
|---|---|
| RMS CLI is pinned | CI installs `rms` from a release tag or release archive. |
| Source provenance exists | The project is a git checkout and CI uses full checkout history. |
| Canonical artifacts validate | `rms validate --root .` passes. |
| Module dependencies compose | `rms compose --root .` passes. |
| Implementations verify | `rms gate --root .` passes and runs declared `rms verify` targets. |
| Release blockers are absent | `rms audit --root . --strict` passes. |
| Evidence is concrete | No active implemented module reports `evidence.placeholder`, `evidence.bootstrap-active`, `evidence.source-unpinned`, or `evidence.semantic-shape-only`. |
| Public compatibility is explicit | Contract or manifest changes include `rms check-compat` evidence or an explicit compatibility decision. |
| Agents can start cold | `AGENTS.md`, `.rms/config.yaml`, and local agent skills are generated or synced from the pinned RMS CLI. |
| Semantic changes are gated | New laws, contracts, artifacts, transformations, protocols, resource lifecycles, authorities, temporal properties, states, commands, events, effects, transitions, semantic roles, public entrypoints, and evidence obligations are introduced through `rms spec apply`, then checked with `rms spec check`. |
| Canonical revision is intact | Strict audit reports `semantic.revision-integrity`; the exact applied record digest matches, superseded records still exist, and a clean commit does not hide direct manifest or history edits after RMS apply. |
| Transition branches are code-backed and replayed | Every declared case exists in the declared transition source, no source-only branch escapes canonical semantics, every lifecycle state is reachable, and strict audit finds matching state/input/destination/source-file/source-branch evidence plus declared workflow events. |
| Transition evidence is truthful | Expected failures use the typed transition rejection channel, and each execution-derived trace record matches its canonical case's exact state change, events, commands, effects, reply, and rejection. |
| Effect execution is trace-complete | Every effectful stateful machine declares exact driver and transition-record functions plus executor semantic functions. Live execution retains complete records, advances from `state_after`, executes `output.effects`, and owns the repeated cycle. |
| Boundary IO is explicit | Inspectable boundary filesystem, process, network, clock, randomness, or persistence work is declared as effects with typed results, atomic protocols, and dedicated executors. Runnable surfaces name exact callables into that machine path. |
| Machine inputs are total | Arithmetic over represented indices, counts, attempts, offsets, lengths, and sequences is checked or bounded; extreme inputs produce explicit rejection instead of overflow, panic, or trap. |
| Message declarations are real | Every declared command, event, effect, and effect-result envelope has a binding-native representation. |
| Cross-module protocols close | Every protocol participant is bound once, every public message has one sender and receiver mapping, and stitched traces preserve identity and causation. |
| Resources close | Every reachable terminal machine path leaves each declared resource in a terminal or transferred state. |
| Elevated authority is contained | Privileged, unsafe, and foreign operations occur only in authority-bound roles behind exact safe facade symbols. |
| Temporal proof matches scope | Finite semantic guarantees use exhaustive or model-checking evidence; runtime and platform bounds use the declared model checker, static analyzer, sanitizer, or benchmark. |

## New Project Flow

Start from product intent. Do not tell the agent RMS internals, desired module names, ADTs, state machines, or file layout.

The onboarding order is `init → authorized bootstrap commit → design → recommended scaffold`.

```bash
rms init . \
  --name <project-name> \
  --purpose "<one sentence>" \
  --context core

rms agent init --target codex --root .
# Optional, for Claude Code scaffolding:
rms agent init --target claude --root .

git add .
git commit -m "Initialize RMS project"
```

The Git commands above are an authorized manual provenance step, not authority granted by RMS. Without Git authority, stop at `bootstrap prepared; provenance baseline pending authorized commit`.

For an existing repository, do not temporarily remove or overwrite project documents. Adopt it explicitly, inspect the per-file report, and commit the resulting bootstrap as its own provenance baseline:

```bash
rms init . \
  --name <project-name> \
  --purpose "<one sentence>" \
  --adopt

git diff -- GLOSSARY.md AGENTS.md .gitignore
git add .
git commit -m "Adopt RMS project structure"
```

The glossary should remain byte-for-byte unchanged. Existing agent instructions and ignore rules should remain intact outside their clearly marked RMS-managed sections. Resolve canonical manifest, malformed-marker, or managed-skill conflicts explicitly; do not bypass adoption with a move-and-restore sequence.

Before choosing modules:

```bash
rms diagnose
rms next --task "<product intent>" --root .
rms design --root . --task "<product intent>"
```

Follow the deterministic scaffold recommendation. If it recommends a recursive capability, use `rms add-capability`; do not collapse it to one module for convenience. A single-module exception requires explicit user intent and canonical justification.

For a product capability that has pure decisions plus a UI, CLI, service, file, process, or network boundary, prefer a recursive capability tree:

```bash
rms add-capability ./modules/<capability> \
  --name <capability> \
  --purpose "<public capability purpose>" \
  --domain-binding rust \
  --boundary-binding js
```

For app, tool, UI, CLI, browser, HTTP, batch, mobile, desktop, executable, local-first, runnable, or smoke-test intents, the boundary module exposes a declared runnable surface. Delegation resolves to an existing role or concrete symbol, the surface declares boundary effects or a precise no-effect justification, and it names an existing usage document plus a smoke command that `rms verify` executes.

Use `rms add-module` only when the module is truly standalone or when the semantic shape is already clear:

```bash
rms add-module ./modules/<module> \
  --name <module> \
  --purpose "<owned responsibility>" \
  --kind library \
  --shape domain-engine \
  --binding rust
```

Then implement inside the generated roles. When behavior changes, update `module.yaml`, contracts, `implementation.yaml`, and evidence only through the applicable `rms spec apply`, `rms machine apply`, or `rms surface apply` command. If the CLI cannot express the required change, report an RMS gap instead of editing canonical artifacts directly.

When product meaning changes, do not ask the agent to hand-create laws, contracts, transitions, or evidence files. Have it run:

```bash
rms spec plan <module.yaml|implementation.yaml> --task "<task>"
rms spec apply <module.yaml|implementation.yaml> --change-yaml '<semantic-change>' --dry-run
rms spec apply <module.yaml|implementation.yaml> --change-yaml '<semantic-change>'
rms spec check <module.yaml|implementation.yaml>
```

Inspect `final_machine` before source edits. Product variants must replace generic scaffold cases, every transition must name its semantic case, and every ordering/safety/bounded/parser/numeric law must have a matching property. Contract operations use `direction: provided` for module-owned surfaces and `direction: required` for consumer expectations; do not publish a dependency contract to make it editable. Every property realization names an exact runner; generated or exhaustive strategies also name a generator, and the runner must execute the generator, semantic operation, and oracle. Spec, machine, and surface apply record their change and reseal canonical semantics. Machine apply does not manufacture replay evidence: a declared producer must call the real transition-record path, and `rms trace run --record` is the only supported way to update active bundles. Direct changes to canonical semantics invalidate the seal and fail strict audit.

Before implementation, also classify the universal system semantics: versioned artifacts and transformations, ordered cross-module protocols, resource ownership and closure, privileged/unsafe/foreign authority, and always/eventually/bounded temporal guarantees. Declare only the structures the product actually needs, but never leave these concerns implicit when correctness depends on them.

## Existing Project Flow

Adopt one boundary at a time.

1. Pick one owner with real invariants, effects, replaceability pressure, or public contract risk.
2. Add or update the system/context/module manifests.
3. Publish only the public contracts that other modules or users may depend on.
4. Bind implementation symbols in `implementation.yaml`.
5. Add the smallest evidence that proves the declared promises.
6. Run the production gates before expanding to another boundary.

Do not model every folder. RMS modules are semantic ownership boundaries, not package directories.

## Agent Workflow

In a fresh agent thread, provide only:

- working directory;
- product or change intent;
- “use RMS CLI/project guidance.”

The project should carry the rest through `AGENTS.md`, local skills, manifests, contracts, and CLI checks.

For changes:

```bash
rms diagnose
rms next --task "<task>" --root .
rms route <module.yaml> --task "<task>"
rms context <module.yaml> --task "<task>"
rms implement <module.yaml> --task "<task>"
```

`rms next` is prospective and read-only. It must not infer the task lane from an unrelated working-tree diff, execute providers or checks, mutate fixtures, or guess through tied owners.

If public meaning changes:

```bash
rms evolve-contract <module.yaml> --task "<task>"
rms check-compat <old-module.yaml> <new-module.yaml>
```

Before completion, preserve `focused checks → gate → authorized candidate commit → strict audit`:

```bash
# Focused native and RMS checks for the changed promise:
rms validate --root .
rms compose --root .
# For each implementation with executable proofs:
rms trace run <implementation.yaml> --profile smoke --record
rms trace run <implementation.yaml> --profile smoke
rms property run <implementation.yaml> --profile smoke
rms gate --root .
git add .
git commit -m "Implement RMS candidate"
rms audit --root . --strict
```

Completion is binary: focused proof must pass before gate, gate must exit zero before the authorized candidate commit, and strict audit must exit zero after it. Failed checks are blockers, not manual notes. Without Git authority, stop at `candidate prepared; strict audit pending authorized commit`.

## Evidence Rules

Evidence must name:

- promise proved;
- success and relevant failure cases;
- command or tool used;
- source revision or artifact identity;
- trace bundle, replay result, or first-bad-transition proof when behavior depends on lifecycle order.
- stitched system trace and first-bad-handoff result when behavior crosses module boundaries;
- artifact compatibility, resource closure, authority containment, and temporal realization results when those semantics are declared.

During work in progress, evidence may be temporary. Before production:

- commit the project;
- replace “local workspace,” “before first git commit,” “source revision: unknown,” and scaffold language;
- rerun `rms audit --root . --strict`.

Strict audit intentionally fails outside a git checkout or when active evidence is unpinned. It executes declared deterministic smoke proofs as trusted project code, compares regenerated traces and reusable packages with committed artifacts, and fails if a proof command mutates a production file. Inspect the command plan first with `rms audit --root . --strict --dry-run` when needed; dry-run cannot produce a production pass.
Strict audit also fails when production-relevant implementation, manifest, contract, evidence, role, or agent-guidance files are dirty or untracked. Commit the production candidate before claiming `rms audit --root . --strict` as release evidence.

Repository-root strict audit treats checked-in `examples/` as illustrative by default. Use `rms audit --root . --strict --include-examples` when examples are part of the production claim, or audit an examples subdirectory directly when that example is the target.

## CI Gate

Use the copyable GitHub Actions template at:

```text
templates/ci/github-actions-rms-project.yml
```

Required CI commands:

```bash
rms diagnose
rms validate --root .
rms compose --root .
rms gate --root .
rms audit --root . --strict
```

Add project-native checks, such as `cargo test`, `swift test`, `npm test`, simulator tests, integration tests, or security scans, according to the implementation bindings and production risk.

## Release Decision

Use this decision table before production deployment:

| Result | Meaning |
|---|---|
| `rms validate --root .` fails | Canonical artifacts are invalid. Do not release. |
| `rms compose --root .` fails | Module dependencies or exports do not compose. Do not release. |
| `rms gate --root .` fails | Declared checks or compatibility obligations are not satisfied. Do not release. |
| `rms audit --root . --strict` fails | Production readiness evidence is incomplete or unpinned. Do not release. |
| All pass, but native production checks fail | RMS structure is sound; implementation is not release-ready. Do not release. |
| All pass | RMS production-pilot gate is satisfied. Continue with normal product, security, and operational release approval. |

## RMS Release Pinning

For production pilot projects, pin one RMS version in CI and agent bootstrap docs.

Recommended:

```bash
cargo install \
  --git https://github.com/reliable-modular-systems/reliable-modular-systems \
  --tag <rms-version-tag> \
  rms \
  --locked
```

Release archives are preferred for end users. Source-tag installs are acceptable for CI when the tag is immutable and reviewed.

After upgrading RMS in a project:

```bash
rms agent sync --target codex --root .
rms agent sync --target claude --root .
# Only when the task and host policy authorize the synchronization commit:
git add AGENTS.md CLAUDE.md .agents .claude .rms
git commit -m "Sync RMS agent guidance"
```

Only sync targets that the project uses.

## What RMS Does Not Claim

RMS does not prove:

- business requirements are correct;
- code has no defects;
- performance, privacy, safety, or security requirements are satisfied unless explicitly modeled and evidenced;
- external systems behave correctly;
- generated code is production-quality without review.

RMS makes structure, ownership, contracts, effects, transitions, and evidence inspectable enough that humans and agents can change software with less architectural drift.

## Done Criteria

A production pilot project is ready to use RMS as its architecture and agent gate when:

- CI runs the RMS gate and strict audit on every pull request;
- the initial module tree follows the deterministic `rms design` recommendation made after the authorized bootstrap provenance commit;
- every implemented module has concrete, source-pinned evidence;
- agents can start from `AGENTS.md` plus RMS CLI/project guidance without hidden conversation context;
- the release owner treats strict audit failures as blockers.

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
| Semantic changes are gated | New laws, contracts, states, commands, events, effects, transitions, semantic roles, public entrypoints, and evidence obligations are introduced through `rms spec apply`, then checked with `rms spec check`. |

## New Project Flow

Start from product intent. Do not tell the agent RMS internals, desired module names, ADTs, state machines, or file layout.

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

Before choosing modules:

```bash
rms diagnose
rms design --root . --task "<product intent>"
```

For a product capability that has pure decisions plus a UI, CLI, service, file, process, or network boundary, prefer a recursive capability tree:

```bash
rms add-capability ./modules/<capability> \
  --name <capability> \
  --purpose "<public capability purpose>" \
  --domain-binding rust \
  --boundary-binding js
```

For app, tool, UI, CLI, browser, HTTP, batch, mobile, desktop, executable, local-first, runnable, or smoke-test intents, the boundary module should expose a declared runnable surface in `architecture.surfaces`. Use `rms surface apply` or `rms spec apply` before adding the entrypoint, and do not leave those intents as library-only unless the product explicitly asks for a library.

Use `rms add-module` only when the module is truly standalone or when the semantic shape is already clear:

```bash
rms add-module ./modules/<module> \
  --name <module> \
  --purpose "<owned responsibility>" \
  --kind library \
  --shape domain-engine \
  --binding rust
```

Then implement inside the generated roles. Update `module.yaml`, contracts, `implementation.yaml`, and evidence when behavior changes.

When product meaning changes, do not ask the agent to hand-create laws, contracts, transitions, or evidence files. Have it run:

```bash
rms spec plan <module.yaml|implementation.yaml> --task "<task>"
rms spec apply <module.yaml|implementation.yaml> --change-yaml '<semantic-change>'
rms spec check <module.yaml|implementation.yaml>
```

`rms spec apply` records the exact applied semantic-change object under `verification/changes/`. Use `set`, `remove`, and `supersedes` to revise semantics instead of hand-editing manifests or rewriting old change records; strict audit treats superseded records as history and active records as reflection obligations. Use `rms machine plan/apply/check` only for focused inner-machine edits when laws, public contracts, and evidence obligations are already correct. Provider plans are advisory until `rms spec apply` or `rms machine apply` reflects them in canonical artifacts. Agents may edit declared role bodies and private pure helpers inside pure role files. IO belongs in declared adapter, port, or effect-executor roles as effects plus effect results.

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
rms route <module.yaml> --task "<task>"
rms context <module.yaml> --task "<task>"
rms implement <module.yaml> --task "<task>"
```

If public meaning changes:

```bash
rms evolve-contract <module.yaml> --task "<task>"
rms check-compat <old-module.yaml> <new-module.yaml>
```

Before completion:

```bash
rms validate --root .
rms compose --root .
rms gate --root .
rms audit --root . --strict
```

## Evidence Rules

Evidence must name:

- promise proved;
- success and relevant failure cases;
- command or tool used;
- source revision or artifact identity;
- trace bundle, replay result, or first-bad-transition proof when behavior depends on lifecycle order.

During work in progress, evidence may be temporary. Before production:

- commit the project;
- replace “local workspace,” “before first git commit,” “source revision: unknown,” and scaffold language;
- rerun `rms audit --root . --strict`.

Strict audit intentionally fails outside a git checkout or when active evidence is unpinned.
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
- the initial module tree was generated from product intent or manually reflected into canonical artifacts;
- every implemented module has concrete, source-pinned evidence;
- agents can start from `AGENTS.md` plus RMS CLI/project guidance without hidden conversation context;
- the release owner treats strict audit failures as blockers.

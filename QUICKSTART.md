# Quickstart

This path proves the RMS workbench from a fresh source checkout. It is written for a user or agent that needs to install the CLI, understand one module, generate derived navigation, and run the release gate.

## Prerequisites

- Rust 1.89 or newer.
- A checkout of this repository.
- Optional: Codex CLI, only when using `--provider codex` or `--ai`.

## Install

For release users, download the platform archive from:

```text
https://github.com/reliable-modular-systems/reliable-modular-systems/releases
```

For source users:

```bash
cargo install --path tooling/rust/rms
```

For contributors who do not want to install:

```bash
cargo run -p rms -- --help
```

## First 10 Minutes

Run deterministic readiness:

```bash
rms diagnose
rms diagnose --json
rms next --task "add a validated public command" --root examples/minimal
```

`rms next` is a read-only, prospective work prescription. It selects an owner only when the canonical artifacts support a unique choice, reports ambiguity instead of guessing, and does not execute providers, verification, edits, or Git commands.

Validate the smallest example:

```bash
rms validate --root examples/minimal
rms compose --root examples/minimal
```

Inspect and explain a module:

```bash
rms inspect examples/minimal/module.yaml
rms explain examples/minimal/module.yaml
rms explain "How does this module work?" --root examples/minimal
```

Generate a local atlas:

```bash
rms atlas examples/minimal/module.yaml \
  --root examples/minimal \
  --output dist/rms-atlas/minimal \
  --force
```

Open `dist/rms-atlas/minimal/index.html` in a browser. The atlas is derived evidence; it does not replace `module.yaml`, contracts, or verification files.

Render agent workbench prompts without calling an AI provider:

```bash
rms plan examples/minimal/module.yaml \
  --root examples/minimal \
  --task "add a validated public command"

rms implement examples/minimal/module.yaml \
  --root examples/minimal \
  --task "add a validated public command" \
  --record

rms run latest --root examples/minimal
```

Provider execution is explicit:

```bash
rms config init
rms explain "How does this module work?" \
  --root examples/minimal \
  --provider codex
```

Do not use `--provider` or `--ai` unless the user intentionally wants an external model run. Provider execution defaults to a 900 second timeout; use `--provider-timeout-seconds <seconds>` only when a longer bounded run is intentional.

## Create A New RMS System

The onboarding order is `init → authorized bootstrap commit → design → recommended scaffold`.

```bash
rms init ./my-system \
  --name my-system \
  --purpose "Build reliable modular software" \
  --context core
```

`rms init` creates the canonical system files plus the local agent/workbench surface a fresh project needs:

```text
system.yaml
context-map.yaml
GLOSSARY.md
AGENTS.md
.rms/config.yaml
.agents/skills/
.gitignore
```

The authorized bootstrap commit establishes the provenance baseline before product design, but only when the task and host policy authorize Git writes:

```bash
git -C ./my-system add .
git -C ./my-system commit -m "Initialize RMS project"
```

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness. Without commit authority, stop at `bootstrap prepared; provenance baseline pending authorized commit`.

Run design before choosing the first module tree:

```bash
rms next --task "<product intent>" --root ./my-system
rms design --root ./my-system --task "<product intent>"
```

Follow the deterministic design recommendation and choose one scaffold path, not both.

For a genuinely standalone module:

```bash
rms add-module ./my-system/modules/widget \
  --name widget \
  --purpose "Own validated widgets" \
  --kind library \
  --shape domain-engine \
  --binding rust
```

Alternatively, for a capability that splits invariant-bearing decisions from a UI, CLI, service, or other boundary, use the recommended recursive tree:

```bash
rms add-capability ./my-system/modules/tic-tac-toe \
  --name tic-tac-toe \
  --purpose "Expose playable Tic-Tac-Toe" \
  --domain-binding rust \
  --boundary-binding js
```

Accept the neutral default child paths unless the product already has meaningful child names.

The generated module includes `module.yaml`, `README.md`, `contracts/README.md`, verification guidance directories, and the requested implementation binding. Use `--binding executable` for opaque command-backed surfaces when Rust, Swift, or JS static checks are not the right fit.

```bash
rms validate --root ./my-system
rms compose --root ./my-system
rms route ./my-system/modules/tic-tac-toe/module.yaml \
  --root ./my-system \
  --task "change invalid move rules"
```

## Release Proof

Run the same gate used by CI and release publication:

```bash
rms release check --root .
```

The gate builds and smoke-tests the release-mode `rms` binary, copies it into a temporary PATH install for a clean-room smoke, validates canonical artifacts, verifies the `rms-cli` implementation binding, checks examples, packages modules, checks Cargo packaging, and verifies packaged Codex skills. It does not invoke optional AI providers.

## Production Pilot Gate

For a production-intended project, continue from quickstart to `PRODUCTION.md`.

Use the completion order `focused checks → gate → authorized candidate commit → strict audit`:

```bash
rms validate --root .
rms compose --root .
# Run the focused native, spec, machine, surface, property, and trace checks
# required by the changed promise.
rms gate --root .
git add .
git commit -m "Implement RMS candidate"
rms audit --root . --strict
```

The candidate commit is a manual authorization step, not authority granted by RMS. If commits are not authorized, stop at `candidate prepared; strict audit pending authorized commit`. Strict audit is intentionally stronger than local validation: it fails unknown source revision, unpinned evidence, scaffold evidence, missing trace bundles, and other production-readiness blockers. Use `templates/ci/github-actions-rms-project.yml` as the starting CI gate for downstream projects.

## Done

The quickstart has succeeded when:

- `rms diagnose` runs;
- `rms next` returns a deterministic prospective prescription without changing the fixture;
- `rms validate --root examples/minimal` passes;
- `rms explain` renders an intelligible module explanation;
- `rms atlas` writes `atlas.json` and `index.html`;
- `rms implement ... --record` writes a run record;
- `rms release check --root .` passes in a source checkout.

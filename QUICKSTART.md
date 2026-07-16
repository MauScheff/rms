# Quickstart

This path installs RMS, uses its five-command surface, and proves a change without loading the specialist command catalog into the main workflow.

## Install

Install the current CLI from this checkout:

```bash
cargo install --locked --path tooling/rust/rms
```

Prebuilt archives for tagged versions are published on the [GitHub releases page](https://github.com/MauScheff/rms/releases); the source checkout may be newer than the latest tag.

Contributors can run the source binary without installing:

```bash
cargo run -p rms -- --help
```

Source builds require Rust 1.89 or newer. External providers are optional. `next --ai` and `design --ai` use the configured provider only for recorded read-only typed-fact extraction; deterministic RMS policy remains the architecture authority.

## First 10 Minutes

This section uses the examples in the source checkout. If you installed only a release archive, skip to [Create a New RMS System](#create-a-new-rms-system).

Check local readiness:

```bash
rms check --environment --root .
```

Ask RMS for a prospective prescription:

```bash
rms next "inspect the example module" --root examples/minimal --ai
```

The compact response gives the outcome, up to three reasons, one immediate action, and the done condition. It does not edit the fixture, invoke a provider, run verification, or grant Git authority.

Ask a focused question:

```bash
rms explain "What does this module own?" \
  --module examples/minimal/module.yaml
```

Add `--details` only when the compact answer is insufficient. Add `--json` for the versioned `rms.surface/v2` agent envelope.

Check canonical validity and composition:

```bash
rms check --root examples/minimal
```

Explore the derived semantic graph:

```bash
rms view --root examples/minimal
```

Use `rms help --all` only when a selected skill or detailed report prescribes a specialist command.

## Create a New RMS System

The onboarding order is:

```text
init → authorized bootstrap commit → design → recommended scaffold
```

Initialize the system:

```bash
rms init ./my-system \
  --name my-system \
  --purpose "Build reliable modular software" \
  --context core
```

For an existing repository with project-owned documents, adopt it explicitly:

```bash
rms init . --adopt \
  --name my-system \
  --purpose "Build reliable modular software"
```

Adoption preserves existing content, creates or refreshes only RMS-managed sections and artifacts, and sets `workspace.coverage: progressive`. Fresh systems use `complete` coverage.

Initialization creates:

```text
system.yaml
context-map.yaml
GLOSSARY.md
AGENTS.md
.rms/config.yaml
.agents/skills/
.gitignore
```

When the task and host policy authorize Git writes, create the authorized bootstrap commit as the provenance baseline before design:

```bash
git -C ./my-system add .
git -C ./my-system commit -m "Initialize RMS project"
```

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness. Without commit authority, stop at `bootstrap prepared; provenance baseline pending authorized commit`.

Ask for the product workflow. The coding agent should extract the typed model; `--ai` is the convenient isolated equivalent:

```bash
rms next "<product intent>" --root ./my-system --ai
rms design --root ./my-system --task "<product intent>" --ai
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

Alternatively, when design recommends a recursive public capability:

```bash
rms add-capability-tree ./my-system/modules/tic-tac-toe \
  --name tic-tac-toe \
  --purpose "Expose playable Tic-Tac-Toe" \
  --domain-binding rust \
  --boundary-binding js \
  --surface browser
```

Accept the neutral default child paths unless the product already has meaningful child names. Do not run both scaffold commands by default.

After scaffolding, ask `next` again. It will prescribe the canonical declaration, declared implementation roles, focused evidence, and completion path for the actual task.

A reusable standalone module publishes its first capability through `rms spec apply` with `contracts.add kind: capability direction: provided` and a matching `public_behavior_bindings` entry. This changes semantics without creating directories. Use `rms add-capability-tree` only when typed design recommends recursive topology.

## Complete a Change

Follow the focused checks selected by `next` and the task-specific skill. Then use the two stable completion modes:

```bash
rms check --changes --root .
# Authorized manual candidate commit, when host policy allows it.
rms check --committed --root .
```

The order is `focused checks → check --changes → authorized candidate commit → check --committed`.

Candidate commits are manual authorization steps, not executable RMS actions. If commits are not authorized, stop at `candidate prepared; strict audit pending authorized commit`. The committed check runs strict audit against the clean candidate and fails incomplete, unpinned, placeholder, or drifted production evidence.

For a production-intended downstream project, continue with [PRODUCTION.md](PRODUCTION.md).

## Maintainer Release Proof

Repository maintainers run the publication gate from the source checkout:

```bash
rms release check --root .
```

This specialist gate builds and smoke-tests the release binary, validates canonical artifacts, verifies examples and packages, checks Cargo packaging, and confirms embedded skills, generated guidance, and integration distributions. It does not invoke optional providers.

## Done

The quickstart succeeds when:

- `rms --help` shows the five primary commands and help doorway;
- `rms check --environment` reports local readiness;
- `rms next` returns a compact deterministic prescription without changing the fixture;
- `rms explain` answers from canonical evidence;
- `rms check --root examples/minimal` passes;
- `rms view` can project the example without becoming semantic authority;
- `rms help --all` makes specialist commands discoverable;
- the applicable change or maintainer release check passes.

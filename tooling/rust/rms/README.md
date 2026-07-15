# `rms`

`rms` is the Rust reference CLI for Reliable Modular Systems. It projects a small human and agent interface over deterministic semantic, structural, verification, packaging, and release engines.

This directory is itself an RMS module bundle:

```text
module.yaml
implementation.yaml
contracts/
verification/
```

## Install

Install the current CLI from this checkout:

```bash
cargo install --locked --path tooling/rust/rms
```

Prebuilt archives for tagged versions are published on the [GitHub releases page](https://github.com/MauScheff/rms/releases); the source checkout may be newer than the latest tag.

Run from source without installing:

```bash
cargo run -p rms -- --help
```

## Primary Interface

```text
rms init [OPTIONS] [PATH]
rms next "<intent>" [--root PATH] [--module MODULE] [--json] [--details]
rms explain ["<question>"] [--root PATH] [--module MODULE] [--json] [--details]
rms check [--environment | --changes | --committed] [--root PATH] [--json] [--details]
rms view [OPTIONS]
```

`rms --help` shows the five primary commands and help doorway. `rms help --all` shows grouped specialist commands used by selected skills, rendered context, and maintainers.

Default human output follows `Outcome/Answer → Why → Next → Done when`. `--details` includes complete canonical diagnostics. `--json` emits the versioned `rms.surface/v2` envelope with typed command or manual actions.

## Typical Flow

Check the local environment and ask what to do:

```bash
rms check --environment --root .
rms next "add a validated public command" --root examples/minimal
```

Ask a focused canonical question:

```bash
rms explain "What does this module own?" \
  --module examples/minimal/module.yaml
```

Check the canonical system:

```bash
rms check --root examples/minimal
```

Complete a candidate:

```bash
rms check --changes --root .
# Authorized manual candidate commit, when host policy allows it.
rms check --committed --root .
```

Production order is `focused checks → check --changes → authorized candidate commit → check --committed`.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness. The pending states are `bootstrap prepared; provenance baseline pending authorized commit` and `candidate prepared; strict audit pending authorized commit`.

## Command Semantics

`init` creates or adopts canonical system artifacts and concise managed agent guidance. The onboarding order is `init → authorized bootstrap commit → design → recommended scaffold`.

`next` is prospective and read-only. It classifies repository shape, resolves ownership only from unambiguous evidence, selects the task lane, and returns safely represented steps without editing files, running verification, invoking providers, or granting source-edit or commit authority. Operational repository work may return `no-rms-change`.

`explain` answers from canonical artifacts and renders relevant evidence once. Unsupported questions return `insufficient-evidence`; full inventory is behind `--details`. The primary command never invokes a provider. Explicit provider-backed explanation uses the specialist prompt workflow shown by `rms help --all`.

`check` delegates to existing engines:

| Mode | Purpose |
| --- | --- |
| `--environment` | Environment and detected-skill readiness |
| default | Canonical validation and composition |
| `--changes` | Candidate change gate |
| `--committed` | Strict committed-candidate audit |

`view` is an experimental loopback-only, read-only semantic explorer. It does not edit canonical artifacts or become another semantic source.

Detected skill sources are observable evidence only. The CLI cannot inspect the current thread's injected skill catalog, so runtime activation is unknown and precedence is host-defined.

## Configuration and Providers

Local workbench configuration remains operational input, not project semantics. Provider execution is always explicit and specialist-only. It may produce advisory prompts or recorded runs, but it cannot make canonical changes valid without deterministic apply and checks.

Use `rms help --all` for configuration, prompt, run-record, semantic apply, machine, surface, trace, property, package, agent-distribution, and compatibility commands.

## Maintainer Release Gate

Before publishing or sharing this CLI, repository maintainers run:

```bash
rms release check --root .
```

The release gate builds and smoke-tests the release binary, validates canonical artifacts, verifies the CLI binding and examples, checks packages and Cargo packaging, exercises a clean-room install, and rejects drift in embedded skills, plugin skills, managed local copies, or generated guidance. It does not invoke optional providers.

The root [Quickstart](../../../QUICKSTART.md), [Production guide](../../../PRODUCTION.md), and [Release process](../../../RELEASE.md) provide the user, downstream-production, and maintainer workflows.

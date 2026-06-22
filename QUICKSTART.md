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
```

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

Do not use `--provider` or `--ai` unless the user intentionally wants an external model run.

## Create A New RMS System

```bash
rms init ./my-system \
  --name my-system \
  --purpose "Build reliable modular software" \
  --context core

rms add-module ./my-system/modules/widget \
  --name widget \
  --purpose "Own validated widgets" \
  --kind library \
  --binding rust

rms validate --root ./my-system
rms compose --root ./my-system
```

## Release Proof

Run the same gate used by CI and release publication:

```bash
rms release check --root .
```

The gate builds and smoke-tests the release-mode `rms` binary, copies it into a temporary PATH install for a clean-room smoke, validates canonical artifacts, verifies the `rms-cli` implementation binding, checks examples, packages modules, checks Cargo packaging, and verifies packaged Codex skills. It does not invoke optional AI providers.

## Done

The quickstart has succeeded when:

- `rms diagnose` runs;
- `rms validate --root examples/minimal` passes;
- `rms explain` renders an intelligible module explanation;
- `rms atlas` writes `atlas.json` and `index.html`;
- `rms implement ... --record` writes a run record;
- `rms release check --root .` passes in a source checkout.

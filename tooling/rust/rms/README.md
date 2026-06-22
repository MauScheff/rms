# `rms`

`rms` is the first reference CLI for Reliable Modular Systems. It is both the deterministic validator and the shared human/agent workbench for RMS projects.

The CLI is itself described as an RMS module bundle in this directory:

```text
module.yaml
implementation.yaml
contracts/
verification/
```

Install from a release archive when you want the normal user path:

```text
https://github.com/reliable-modular-systems/reliable-modular-systems/releases
```

Install from this repository when working from source:

```bash
cargo install --path tooling/rust/rms
```

Run without installing:

```bash
cargo run -p rms -- validate --root examples/minimal
```

Common commands:

```bash
rms validate --root examples/minimal
rms inspect examples/minimal/module.yaml
rms explain examples/minimal/module.yaml
rms explain examples/minimal/module.yaml "What does this module own?"
rms explain "How does this module work?" --root examples/minimal
rms diagnose
rms diagnose --json
rms config init
rms plan examples/minimal/module.yaml --task "add a public command"
rms implement examples/minimal/module.yaml --task "add a public command"
rms evolve-contract examples/minimal/module.yaml --task "change command failure semantics"
rms evidence examples/minimal/module.yaml --task "prove invalid examples are rejected"
rms refactor examples/minimal/module.yaml --task "separate decisions from effects"
rms review examples/minimal/module.yaml
rms impact
rms impact HEAD~1..HEAD --json
rms prompt evidence examples/minimal/module.yaml --task "prove invalid examples are rejected"
rms plan examples/minimal/module.yaml --task "add a public command" --record
rms implement examples/minimal/module.yaml --task "add a public command" --ai
rms review examples/minimal/module.yaml --provider codex
rms explain --module examples/minimal/module.yaml "How does this module work?" --ai
rms run list
rms run latest
rms run inspect <run-id>
rms release check --root .
rms context examples/minimal/module.yaml --task "change payment capture behavior"
rms atlas examples/minimal/module.yaml --output dist/rms-atlas/minimal
rms conformance examples/minimal/module.yaml --implementation examples/minimal/implementation.yaml
```

Before publishing or sharing this CLI, run:

```bash
rms release check --root .
```

The quickstart is in `../../../QUICKSTART.md`, the self-hosted walkthrough is in `../../../DOGFOOD.md`, and the release process is in `../../../RELEASE.md` from the repository root.

The workbench prompt commands are advisory by default. They render bounded, versioned prompts for humans or agents. Use `--record` to write `.rms/runs/<run-id>/request.yaml`, `prompt.md`, and `checks.json`. Use `--provider codex` to execute the prompt through `codex exec`, or `--ai` to use `ai.default_provider` from `.rms/config.yaml`, and record `response.md` plus provider logs. The CLI remains intentionally conservative and reports missing evidence explicitly instead of claiming more conformance than the artifacts prove.

Optional `.rms/config.yaml`:

```yaml
ai:
  default_provider: codex
  codex:
    model: gpt-5-codex
    sandbox: read-only
runs:
  directory: .rms/runs
```

# Dogfood Walkthrough

The `rms` CLI is itself an RMS module. This walkthrough uses the workbench on its own module bundle to prove that the same commands used for other projects also guide changes to RMS.

## Target Module

| Role | Path |
|---|---|
| Module manifest | `tooling/rust/rms/module.yaml` |
| Implementation binding | `tooling/rust/rms/implementation.yaml` |
| Public contracts | `tooling/rust/rms/contracts/` |
| Evidence | `tooling/rust/rms/verification/` |
| Source | `tooling/rust/rms/src/main.rs` |

The owning module is `rms-cli`. It owns the command surface, diagnostics, explanations, package reports, workbench prompts, run records, release readiness, and atlas artifacts. The implementation binding declares Rust as the binding and `cargo test --manifest-path Cargo.toml` as the native verification command.

## First Read

Start with deterministic context:

```bash
rms inspect tooling/rust/rms/module.yaml
rms explain tooling/rust/rms/module.yaml
rms explain tooling/rust/rms/module.yaml \
  "How does the release gate protect packaged skills and binaries?"
```

Use this output to identify ownership, public commands, effects, compatibility policy, and verification evidence. If the explanation and manifest disagree, treat that as architectural drift and inspect the canonical artifacts.

## Atlas

Generate a derived map of the module:

```bash
rms atlas tooling/rust/rms/module.yaml \
  --root . \
  --output dist/rms-atlas/rms-cli \
  --force
```

Review:

```text
dist/rms-atlas/rms-cli/atlas.json
dist/rms-atlas/rms-cli/index.html
```

Use the atlas for navigation. Do not edit atlas output as architecture. The source of truth remains `module.yaml`, contracts, implementation binding, and evidence files.

## Change Impact

Before reviewing or implementing a change, classify git impact:

```bash
rms impact --root .
rms impact HEAD~1..HEAD --root . --json
```

Interpretation rules:

| Impact output | Agent action |
|---|---|
| `module-manifest.changed` | Review ownership, public surface, profiles, effects, compatibility, and verification references. |
| `contract.changed` | Classify compatibility before implementation; update evidence. |
| `implementation-binding.changed` | Verify source symbols, dependencies, commands, and binding assumptions. |
| `source.changed` | Run the owning module verification command. |
| `verification-evidence.changed` | Confirm the evidence still proves a manifest promise. |
| `unowned-repository-artifact` | Treat as repository evidence, not semantic authority. |

## Agent Workbench

Render prompts before broad edits:

```bash
rms plan tooling/rust/rms/module.yaml \
  --root . \
  --task "add release artifact smoke coverage"

rms implement tooling/rust/rms/module.yaml \
  --root . \
  --task "add release artifact smoke coverage" \
  --record

rms review tooling/rust/rms/module.yaml \
  --root . \
  --record
```

The prompt output is advisory. The executing agent must still read canonical artifacts, edit the owning module, update contracts or evidence when public meaning changes, and run deterministic checks.

## Verification

For ordinary changes:

```bash
rms validate --root .
rms verify tooling/rust/rms/implementation.yaml
```

For release or distribution changes:

```bash
rms release check --root .
```

The release gate is the strongest local proof lane. It includes release metadata checks, formatting, Rust tests, RMS validation, module verification, composition and compatibility smoke tests, package smoke tests, scaffold roundtrips, release-binary smoke, clean-room PATH install smoke, Cargo packaging, and Codex plugin sync validation.

## Done

A dogfood change is complete when:

- the owning `rms-cli` manifest and public contracts still describe the command surface;
- implementation binding symbols exist in source;
- new generated artifacts are semantically reachable from the module bundle;
- provider-backed commands remain opt-in;
- `rms release check --root .` passes;
- the change summary names compatibility impact and any operational caveats.

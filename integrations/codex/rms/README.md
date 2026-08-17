# Codex Plugin Wrapper

This directory packages the canonical RMS skills for Codex. It is an adapter: the semantic source remains the repository manifests, contracts, `skills/` directory, and shared `rms` CLI.

## Use Locally

Install the neutral CLI first:

```text
https://github.com/MauScheff/rms/releases
```

Or install from a source checkout:

```bash
cargo install --locked --path tooling/rust/rms
```

Then point Codex at this plugin directory or copy it into a Codex plugin marketplace according to the current Codex plugin workflow.

Start software-change work through the narrow doorway:

```bash
rms next "<exact change task>" --root . --ai
rms explain "<question>" --root .
rms check --root .
```

Use native project tools for read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, and discussion that requests no change. If that work reveals a proposed change, stop before editing and route the exact change task through RMS.

Use `rms help --all` for specialist commands. The plugin must not carry a second RMS workflow.

Specialist selection is narrow:

| Need | Command |
| --- | --- |
| Migrate one legacy binding | `rms binding migrate ... --to v0.2 --route-receipt ... --dry-run` |
| Prove transitive purity and authority | `rms structure <implementation.yaml>` |
| Generate inputs for one described machine | `rms property generate <implementation.yaml> --out <assembly>` |
| Check composition without writes | `rms compose --root <root>` |
| Generate a symbolic composed probe | `rms compose --root <root> --output <dir> --dry-run`, then write mode |
| Run bounded discovery | `rms hunt --dry-run`, then a clean-commit hunt |

Generated analyses, assemblies, models, and proof certificates are evidence. They never authorize semantic edits. The packaged `inspect-module`, `implement-change`, `compose-modules`, `verify-module`, and `hunt-bugs` skills contain the detailed decision rules.

Machine-facing doorway reports use `rms.surface/v2`. Detected skill files do not prove runtime activation; `rms diagnose` reports that distinction. Git commits are required evidence, not implied authority, and production proof completes only through the committed RMS check.

## Refresh Skills

The plugin carries a copy of the canonical skills so it can be distributed as a self-contained plugin. Refresh that copy before release:

```bash
./integrations/codex/rms/scripts/sync-skills.sh
```

Do not edit `integrations/codex/rms/skills/` directly unless the same change is made in canonical `skills/`.

After local plugin synchronization, start a new Codex task. The existing task retains the skill catalog injected when it started.

## Validation

From the repository root:

```bash
rms release check --root .
```

The release check validates that packaged plugin skills match canonical `skills/`. The plugin should remain thin. Skills, hooks, and MCP servers should call the shared five-command doorway and task-selected specialist commands rather than implementing private architectural rules.

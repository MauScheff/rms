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

Start project work through the narrow doorway:

```bash
rms next "<intent>" --root .
rms explain "<question>" --root .
rms check --root .
```

Use `rms help --all` for specialist commands. The plugin must not carry a second RMS workflow.

## Refresh Skills

The plugin carries a copy of the canonical skills so it can be distributed as a self-contained plugin. Refresh that copy before release:

```bash
./integrations/codex/rms/scripts/sync-skills.sh
```

Do not edit `integrations/codex/rms/skills/` directly unless the same change is made in canonical `skills/`.

## Validation

From the repository root:

```bash
rms release check --root .
```

The release check validates that packaged plugin skills match canonical `skills/`. The plugin should remain thin. Skills, hooks, and MCP servers should call the shared five-command doorway and task-selected specialist commands rather than implementing private architectural rules.

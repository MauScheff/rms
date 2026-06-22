# Codex Plugin Wrapper

This directory packages the canonical RMS skills for Codex. It is an adapter: the semantic source remains the repository manifests, contracts, `skills/` directory, and shared `rms` CLI.

## Use Locally

Install the neutral CLI first:

```text
https://github.com/reliable-modular-systems/reliable-modular-systems/releases
```

Or install from a source checkout:

```bash
cargo install --path tooling/rust/rms
```

Then point Codex at this plugin directory or copy it into a Codex plugin marketplace according to the current Codex plugin workflow.

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

The release check validates that packaged plugin skills match canonical `skills/`. The plugin should remain thin. Skills, hooks, and MCP servers should call the shared `rms` CLI rather than implementing private architectural rules.

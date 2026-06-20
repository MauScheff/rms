# Codex Plugin Wrapper

This directory packages the canonical RMS skills for Codex. It is an adapter: the semantic source remains the repository manifests, contracts, and `skills/` directory.

## Use Locally

Install the neutral CLI first:

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
python3 /Users/mau/.codex/skills/.system/plugin-creator/scripts/validate_plugin.py integrations/codex/rms
```

The plugin should remain thin. Hooks and MCP servers may be added later, but they should call the shared `rms` CLI rather than implementing private architectural rules.


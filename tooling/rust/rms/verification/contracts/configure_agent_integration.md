# Contract Evidence: configure-agent-integration

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic agent integration coverage.

Executable coverage:

- `agent_integration_init_and_sync_manage_project_local_guidance` verifies `rms agent init` creates project-local `AGENTS.md`, `.agents/skills`, and `.rms/config.yaml`; refuses accidental overwrite; `rms agent sync` refreshes stale generated guidance and skills; and sync preserves existing workbench config.
- The same test verifies diagnosis data reports project-local skills as present and records that the Codex plugin is not required.
- `agent_integration_scaffolds_claude_project_guidance` verifies the Claude target creates `CLAUDE.md`, installs RMS skills under `.claude/skills`, and records that a Claude plugin is not required.
- `agent_plugin_install_packages_codex_plugin_marketplace_entry` verifies optional Codex plugin installation writes a self-contained plugin package, skill files, and personal marketplace entry without requiring global mutation during tests.
- `agent_plugin_install_rejects_unsupported_targets` verifies plugin installation is explicit about unsupported targets such as Claude.

The command uses embedded RMS guidance and skill assets. It does not install global plugins, invoke providers, or create module-specific architecture.

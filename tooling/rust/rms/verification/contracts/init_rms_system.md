# Contract Evidence: init-rms-system

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic scaffold coverage.

Executable coverage:

- `init_scaffold_generates_valid_system_artifacts` verifies generated `system.yaml` and `context-map.yaml` validate, `AGENTS.md` contains the RMS operating rule, `.rms/config.yaml` parses as workbench config with Codex read-only defaults, `.gitignore` excludes RMS run records, and `.agents/skills` contains the embedded RMS workflow skills.
- `embedded_init_agent_skills_match_canonical_source_when_available` verifies the embedded scaffolded skill assets match the repository canonical `skills/` source when tests run from a source checkout.
- `rms release check --root .` runs a scaffold roundtrip that initializes a new RMS system, adds Rust and Swift modules, validates and composes the scaffold, and verifies the generated Rust binding.

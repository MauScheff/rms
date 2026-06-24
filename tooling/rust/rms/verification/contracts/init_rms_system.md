# Contract Evidence: init-rms-system

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic scaffold coverage.

Executable coverage:

- `init_scaffold_generates_valid_system_artifacts` verifies generated `system.yaml` and `context-map.yaml` validate, `AGENTS.md` contains the RMS operating rule, semantic design/scaffold guidance, representation and transition guidance, `.rms/config.yaml` parses as workbench config with Codex read-only defaults plus module-scoped provider edit guidance and timeout guidance, `.gitignore` excludes RMS run records, and `.agents/skills` contains the embedded RMS workflow skills.
- `embedded_init_agent_skills_match_canonical_source_when_available` verifies the embedded scaffolded skill assets match the repository canonical `skills/` source when tests run from a source checkout.
- `rms release check --root .` runs a scaffold roundtrip that initializes a new RMS system, adds Rust, Swift, and Boundary-profile executable modules, validates and composes the scaffold, verifies the generated executable binding, and verifies the generated Rust binding.

# Contract Evidence: init-rms-system

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic scaffold coverage.

Executable coverage:

- `init_scaffold_generates_valid_system_artifacts` verifies generated `system.yaml` and `context-map.yaml` validate, `AGENTS.md` contains the typed-intent architecture gate and binary completion sequence, `.rms/config.yaml` records complete workspace coverage with Codex read-only defaults, `.gitignore` excludes RMS run records, `.agents/skills` contains the embedded workflows, and a standalone target becomes a Git worktree.
- `init_reuses_parent_git_worktree_without_nested_repository` verifies initialization inside an existing worktree reuses its provenance root and does not create a nested `.git` directory.
- `init_adopt_preserves_existing_documents_and_creates_only_missing_artifacts`, `init_adopt_accepts_an_already_compatible_rms_workspace`, `init_without_adopt_refuses_existing_documents_before_writing`, and `init_adopt_conflict_preflight_writes_nothing` verify explicit progressive adoption, project-document preservation, safe managed-config upgrade, strict-mode refusal, and collision preflight atomicity.
- `adoption_complete_rejects_unowned_production_paths` proves an adopted workspace cannot claim complete coverage while production paths remain outside RMS ownership.
- `embedded_init_agent_skills_match_canonical_source_when_available` verifies the embedded scaffolded skill assets match the repository canonical `skills/` source when tests run from a source checkout.
- `rms release check --root .` runs a scaffold roundtrip that initializes a new RMS system, adds Rust, Swift, and Boundary-profile executable modules, validates and composes the scaffold, verifies the generated executable binding, and verifies the generated Rust binding.

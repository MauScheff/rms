# Contract Evidence: gate-rms-change

Covered by `cargo test --manifest-path Cargo.toml`.

Executable coverage:

- `gate_plan_skips_unrelated_paths` verifies paths without RMS impact produce no executable checks.
- `gate_plan_runs_verify_for_source_changes` verifies source changes select validation and the affected implementation verification command.
- `gate_plan_runs_composite_verify_for_parent_evidence_changes` verifies parent composite evidence changes select validation and the composite module rollup verification command without requiring a parent implementation binding.
- `gate_plan_marks_contract_changes_for_review_and_compatibility` verifies public contract changes select validation, composition, review, and compatibility obligations.
- `gate_runs_full_deterministic_checks_outside_git_repository` verifies non-git roots run deterministic checks but return failure because no source revision exists.
- `gate_structural_preflight_blocks_missing_semantic_revision` verifies strict semantic and structural blockers are gate failures before commit.
- `gate_preflight_ignores_only_final_worktree_provenance` verifies preflight excludes only final dirty/untracked/source-revision cleanliness checks and keeps semantic blockers authoritative.
- `compose_unions_external_dependencies_from_discovered_systems` verifies repository-level composition can satisfy required capabilities from any discovered system manifest before the gate runs `rms compose --root`.

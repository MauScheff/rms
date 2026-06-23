# Contract Evidence: gate-rms-change

Covered by `cargo test --manifest-path Cargo.toml`.

Executable coverage:

- `gate_plan_skips_unrelated_paths` verifies paths without RMS impact produce no executable checks.
- `gate_plan_runs_verify_for_source_changes` verifies source changes select validation and the affected implementation verification command.
- `gate_plan_marks_contract_changes_for_review_and_compatibility` verifies public contract changes select validation, composition, review, and compatibility obligations.
- `gate_reports_friendly_message_outside_git_repository` verifies non-git roots fail with RMS guidance to initialize git or run deterministic checks directly, rather than exposing raw git usage text.
- `compose_unions_external_dependencies_from_discovered_systems` verifies repository-level composition can satisfy required capabilities from any discovered system manifest before the gate runs `rms compose --root`.

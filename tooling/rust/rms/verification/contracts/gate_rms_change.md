# Contract Evidence: gate-rms-change

Covered by `cargo test --manifest-path Cargo.toml`.

Executable coverage:

- `gate_plan_skips_unrelated_paths` verifies paths without RMS impact produce no executable checks.
- `gate_plan_runs_verify_for_source_changes` verifies source changes select validation and the affected implementation verification command.
- `gate_plan_marks_contract_changes_for_review_and_compatibility` verifies public contract changes select validation, composition, review, and compatibility obligations.

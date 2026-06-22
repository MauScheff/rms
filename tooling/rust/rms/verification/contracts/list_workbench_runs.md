# Contract Evidence: list-workbench-runs

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic run-record fixtures that verify `rms` can list generated run records without mutating them.

Executable coverage:

- `run_list_and_inspect_read_generated_records` verifies listing reads generated run records.
- `prompt_options_use_configured_ai_defaults` verifies the configured run directory can be resolved from `.rms/config.yaml`.

# Contract Evidence: inspect-workbench-run

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic run-record fixtures that verify `rms` can inspect request metadata, files, checks, and response content from one run directory.

Executable coverage:

- `run_list_and_inspect_read_generated_records` verifies inspection reads generated run record metadata and response content.
- `prompt_options_use_configured_ai_defaults` verifies run ids can be resolved through the configured run directory when no explicit `--run-root` is supplied.

# Contract Evidence: latest-workbench-run

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic run-record ordering coverage.

Executable coverage:

- `latest_run_dir_uses_newest_run_id` verifies the newest generated run id is selected.
- `run_list_and_inspect_read_generated_records` verifies run inspection renders request metadata, checks, and response content without mutating run artifacts.

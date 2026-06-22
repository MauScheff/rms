# Contract Evidence: init-workbench-config

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic config generation coverage.

Executable coverage:

- `config_init_writes_defaults_and_refuses_overwrite` verifies generated `.rms/config.yaml` parses as workbench config, writes provider/model/run-record defaults, and refuses overwrite unless forced.
- `diagnose_report_includes_config_and_serializes_to_json` verifies generated config fields are visible as readiness evidence rather than module semantics.

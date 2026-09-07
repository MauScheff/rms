# Contract Evidence: run-trace-producers

Promise:

- Contract `run-trace-producers` executes declared producers, validates generated bundles, records only under `--record`, compares otherwise, supports dry-run, and enforces a timeout.

Scenario:

- Record a generated JS bundle and compare it with fresh execution.
- Alter committed evidence and rerun comparison.
- Configure a machine-backed capability to claim invocation-record observations and attempt `--record`.
- Run an opaque producer that exceeds its timeout.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml trace_run_records_and_detects_committed_bundle_drift`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml failed_trace_preflight_does_not_replace_committed_evidence`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml transition_backed_capability_rejects_invocation_record_observation`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml trace_run_reports_producer_timeout`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_module_scaffold_generates_valid_binding_artifacts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml swift_module_scaffold_generates_valid_binding_artifacts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml js_boundary_adapter_scaffold_separates_representation_parser_and_adapters`

Expected result:

- Rust, Swift, JavaScript, and Python producers record valid transition-derived bundles.
- Normal comparison reports drift after committed evidence changes.
- A command or capability observed through its declared transition-record trace producer uses transition records. Invocation records remain valid for stateless queries and commands observed through another command path.
- A failed producer preflight does not execute the producer. A failed selected-producer suite does not replace committed evidence.
- Timeout terminates the producer process group and reports the exact producer.

Observed result: all focused regressions passed. Source revision is resolved from the committed candidate by strict audit.

# Law Evidence: trace evidence is execution-derived

Promise:

- Law `trace-evidence-is-execution-derived` requires active transition evidence to be regenerated from the implementation's declared transition-record path.

Scenario:

- Reject a complete trace bundle with no smoke producer.
- Reject a producer that calls `transition_record` but serializes a static declaration instead of returned record fields.
- Record a valid bundle, alter the committed record order, and compare it with fresh execution.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml production_trace_bundle_requires_a_smoke_producer`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml trace_producer_must_serialize_returned_transition_records`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml trace_run_records_and_detects_committed_bundle_drift`

Expected result:

- Missing producers report `trace.producer-missing`.
- Static-copy producers report `trace.producer-bypasses-transition-record`.
- Changed committed evidence reports `trace.generated-bundle-drift`.

Observed result: all focused regressions passed. Source revision is resolved from the committed candidate by strict audit.

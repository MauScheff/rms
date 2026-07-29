# Contract Evidence: inspect-trace-bundle

Covered by `cargo test --manifest-path tooling/rust/rms/Cargo.toml`, including local trace-bundle tests.

Executable coverage:

- `trace_bundle_replays_local_transition_records` verifies a JSON/YAML-compatible `rms/trace-bundle/v0.1` file checks cleanly and `rms trace show` reconstructs the recorded timeline without executing module code.
- `trace_bundle_diagnoses_first_bad_transition` verifies a discontinuous local journal produces `trace.timeline-discontinuity` and identifies the first bad transition.
- `trace_bundle_reports_declared_first_bad_transition` verifies `rms trace diagnose` reports valid bundle-level `first_bad_transition` metadata even when the recorded timeline is otherwise structurally clean.
- `trace_bundle_rejects_out_of_range_declared_first_bad_transition` verifies invalid `first_bad_transition.index` metadata fails trace checking.

The command is intentionally CLI-first. It inspects recorded local evidence only; it does not introduce or require a runtime, router, journal service, effect dispatcher, or shared execution framework.

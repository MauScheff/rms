# Contract Evidence: inspect-trace-bundle

Covered by `cargo test --manifest-path tooling/rust/rms/Cargo.toml`, including local trace-bundle tests.

Executable coverage:

- `trace_bundle_replays_local_transition_records` verifies a JSON/YAML-compatible `rms/trace-bundle/v0.1` file with transition records checks cleanly and reconstructs the final recorded state without executing module code.
- `trace_bundle_diagnoses_first_bad_transition` verifies a discontinuous local journal produces `trace.timeline-discontinuity` and identifies the first bad transition.

The command is intentionally CLI-first. It inspects recorded local evidence only; it does not introduce or require a runtime, router, journal service, effect dispatcher, or shared execution framework.

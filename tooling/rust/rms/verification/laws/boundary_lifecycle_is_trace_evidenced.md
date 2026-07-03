# Law Evidence: boundary lifecycle is trace evidenced

Promise:

- `boundary-lifecycle-is-trace-evidenced`
- Boundary-machine lifecycle states are evidenced by replayable transition traces.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml strict_audit_boundary_machine_requires_lifecycle_trace_coverage`

Expected result:

- Strict audit fails when declared boundary lifecycle states are not present in declared trace bundles.

Source revision:

- Recorded by git commit before a production claim.

# Law Evidence: strict audit regenerates smoke proofs

Promise:

- Law `strict-audit-regenerates-smoke-proofs` requires deterministic smoke traces and properties to execute during strict audit without changing production files.

Scenario:

- Execute a property command that modifies a declared source file.
- Execute a trace producer whose descendant process exceeds the configured timeout.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml strict_proof_execution_rejects_production_file_mutation`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml trace_run_reports_producer_timeout`

Expected result:

- Source mutation reports `proof.command-mutated-source`.
- The timed-out process group is terminated and reports `proof.command-timeout`.

Observed result: both focused regressions passed. Source revision is resolved from the committed candidate by strict audit.

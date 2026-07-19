# Contract evidence: check-rms-project v2

Compact JSON and human output carry dedicated coverage and proof projections. Every discovered top-level closure is named, unowned production paths are counted from discovery, and declared property or trace-producer evidence is separated from observed execution.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml check_proof_projection_never_promotes_declarations`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml progressive_coverage_names_every_closure`

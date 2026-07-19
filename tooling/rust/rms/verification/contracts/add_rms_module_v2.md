# Contract evidence: add-rms-module v2

A standalone module scaffold is gated by the exact ready design receipt and reports the accepted receipt ID. The validator runs before `run_add_module` can create a directory.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml design_receipt_binds_exact_scaffold_arguments`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml route_receipt_gate_accepts_only_matching_ready_context`

Only the canonical path, name, purpose, shape, binding, and action combination is accepted.

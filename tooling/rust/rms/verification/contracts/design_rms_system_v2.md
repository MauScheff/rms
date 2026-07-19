# Contract evidence: design-rms-system v2

Every design result persists a run record and receipt. Only `ready` carries a non-empty action and exact scaffold; clarification, missing-model, invalid, and blocked outcomes return semantic exit status 2 and cannot authorize topology mutation.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml design_receipt_binds_exact_scaffold_arguments`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml route_receipt_gate_accepts_only_matching_ready_context`

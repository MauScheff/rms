# Contract evidence: apply-runnable-surface v2

Surface apply, including dry-run, is rejected before candidate construction unless the receipt authorizes `surface-apply` for the exact implementation target and owner context.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml mutator_cli_requires_route_receipts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml route_receipt_gate_accepts_only_matching_ready_context`

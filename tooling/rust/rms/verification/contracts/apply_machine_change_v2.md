# Contract evidence: apply-machine-change v2

Machine apply, including dry-run, resolves and validates a ready receipt before parsing or computing its candidate. The receipt must authorize `machine-apply` for the selected implementation target at the current Git HEAD.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml mutator_cli_requires_route_receipts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml route_receipt_gate_accepts_only_matching_ready_context`

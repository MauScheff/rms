# Contract evidence: add-rms-capability-tree v2

A capability-tree scaffold accepts only a ready design receipt whose canonical scaffold program and arguments exactly equal the requested topology mutation.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml design_receipt_binds_exact_scaffold_arguments`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml mutator_cli_requires_route_receipts`

The regression cases cover changed paths, names, purposes, bindings, surfaces, and action families before any directory is created.

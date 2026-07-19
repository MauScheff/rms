# Contract evidence: add-rms-binding v2

The CLI parser requires a route receipt before an implementation binding can be attached. Validation occurs before `run_add_binding`, and the accepted receipt ID is reported after a successful dry-run or write.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml route_receipt_gate_accepts_only_matching_ready_context`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml mutator_cli_requires_route_receipts`

The tests exercise missing, mismatched, stale, and exact receipt inputs and assert that rejection precedes candidate mutation.

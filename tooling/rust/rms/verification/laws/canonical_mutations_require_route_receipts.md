# Law evidence: canonical mutations require route receipts

The shared validator recomputes the receipt digest and checks schema, issuing RMS version, ready status, canonical repository identity, current Git HEAD, action family, normalized target, exact scaffold, and run-record task/intent/schema digests before a mutator is called.

An explicitly selected legacy implementation blocked only by recognized pre-rc.8 machine probe/initial-state omissions may receive a migration-only ready receipt. That receipt authorizes `machine-apply` for the selected implementation and no other mutator. Any unrelated implementation, semantic, contract, ownership, or repository diagnostic keeps the route blocked; normal machine apply and post-apply validation remain mandatory.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml route_receipt_gate_accepts_only_matching_ready_context`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml mutator_cli_requires_route_receipts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml next_authorizes_only_machine_apply_for_explicit_legacy_binding_migration`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml next_rejects_legacy_migration`

Receipt acceptance grants neither source-edit authority nor Git authority; those authorities remain outside the receipt schema.

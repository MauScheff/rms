# Law evidence: canonical mutations require route receipts

The shared validator recomputes the receipt digest and checks schema, issuing RMS version, ready status, canonical repository identity, current Git HEAD, action family, normalized target, exact scaffold, and run-record task/intent/schema digests before a mutator is called.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml route_receipt_gate_accepts_only_matching_ready_context`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml mutator_cli_requires_route_receipts`

Receipt acceptance grants neither source-edit authority nor Git authority; those authorities remain outside the receipt schema.

# Contract evidence: recommend-next-rms-work v3

Every `rms next` invocation creates an inspectable run and route receipt. Ready receipts bind task, normalized intent, schema and prompt digests, repository and HEAD, lane, owner, actions, and targets; non-ready receipts contain no allowed mutation.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml route_receipt_gate_accepts_only_matching_ready_context`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml intent_extraction_cache_is_deterministic`

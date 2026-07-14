# Property Evidence: protocol-aware executor control flow

Property `protocol-aware-executor-control-flow` covers an atomic executor and an aggregate executor in the same implementation.

Command/tool: `cargo test --manifest-path Cargo.toml aggregate_executor_loop_is_not_rejected_by_unrelated_atomic_protocol`

Observed result: 1 deterministic mixed-protocol fixture passed. The aggregate loop produced no atomicity diagnostic. Atomic executor loop rejection remains covered by the Rust call-graph conformance suite. No counterexample was produced.

Source revision: resolved from the committed candidate by strict audit.

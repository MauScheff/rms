# Law Evidence: protocol-aware executor analysis

Promise: invariant `effect-executor-analysis-respects-protocol-atomicity` requires control-flow analysis to use each exact effect protocol's atomicity.

Command/tool:

- `cargo test --manifest-path Cargo.toml aggregate_executor_loop_is_not_rejected_by_unrelated_atomic_protocol`
- `cargo test --manifest-path Cargo.toml rust_machine_driver_and_effect_protocol_checks_are_enforced`

Expected and observed result: a loop reachable from an atomic executor is rejected, while a declared aggregate executor is not rejected because another protocol is atomic. Both focused fixtures pass.

Source revision: resolved from the committed candidate by strict audit.

# Law Evidence: effect executor semantic authority

Promise:

- `effect-executors-are-semantic-functions`: an exact effect protocol executor has an explicit semantic-function owner rather than existing only as incidental source code.

Scenario:

- Machine apply sees an effect protocol with exact executor symbol `execute_invocation`.
- It creates or updates the matching semantic function with `kind: effect-executor` and `purity: effectful`.
- Structure validation reports `structure.effect-executor-semantic-owner-missing` when a declared protocol lacks that owner.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml machine_apply_declares_effect_executor_semantic_function`

Expected result:

- Effect-executor authority can be discharged by an exact first-class semantic function.
- Transition authority remains restricted to pure transition functions.

Source revision: resolved from the candidate Git commit by strict audit.

# Law Evidence: semantic-function bindings are CLI-applied

Promise:

- `semantic-function-bindings-are-cli-applied`: implementation semantic functions change through a validated, sealed RMS semantic revision rather than direct manifest editing.

Scenario:

- Add a parser semantic function that discharges a parser-authority invariant and points to concrete boundary evidence.
- Replace its binding symbol through `semantic_functions.set`.
- Reject removal while the invariant would lose its authority owner.
- Remove the function and retired invariant together.
- Confirm dry-run reports the final function set without mutating `implementation.yaml`.
- Confirm strict invariant proof recognizes concrete evidence attached to the exact semantic function that discharges the invariant, including a composition-authority machine driver, while a missing evidence file remains a blocker.

Command/tool:

```sh
cargo test --workspace --locked spec_apply_gates_semantic_function_add_set_and_remove
cargo test --workspace --locked invariant_accepts_concrete_evidence_from_its_enforcing_semantic_function
cargo run --manifest-path tooling/rust/rms/Cargo.toml -- validate --root .
```

Expected result:

- Add, set, and coordinated remove operations update and seal `implementation.yaml`.
- A missing implementation, unknown id, duplicate final id, invalid kind or purity, missing discharged promise, unsafe evidence path, or final invariant-authority gap is rejected before writes.
- The active semantic-change record contains the exact function binding and strict audit can compare it with the implementation manifest.
- Strict proof and semantic apply agree on the existing semantic-function evidence representation; no redundant invariant-level evidence alias is required.

Source revision: recorded by git commit or strict audit provenance before production use.

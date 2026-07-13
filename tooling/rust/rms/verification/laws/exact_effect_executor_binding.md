# Exact Effect Executor Binding

Promise: `effect-protocols-bind-exact-executors`

## Scenarios

- Rust, Swift, and JavaScript workflow scaffolds bind each effect protocol to a declared executor role and exact callable symbol.
- A protocol with a missing symbol, wrong role, missing function, incompatible request/result signature, or executor-owned transition is diagnosed deterministically.
- A one-request-one-result executor containing a lifecycle loop is rejected.

## Commands

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml workflow_rust_scaffold_generates_traceable_orchestration_roles
cargo test --manifest-path tooling/rust/rms/Cargo.toml effectful_swift_and_js_scaffolds_bind_machine_drivers_and_exact_executors
cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_boundary_rejects_hidden_imperative_effect_loop_bypassing_machine_driver
```

## Expected Result

Generated implementations pass structure checks with an exact executor for every protocol. Invalid fixtures report the corresponding `structure.effect-protocol-executor-*` or `structure.effectful-control-flow-outside-machine-driver` diagnostic, and strict audit treats it as blocking.

Source provenance: the committed candidate revision containing this evidence and the named tests is resolved by `rms audit --root . --strict`.

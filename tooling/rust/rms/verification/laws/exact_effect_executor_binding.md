# Exact Effect Executor Binding

Promise: `effect-protocols-bind-exact-executors`

## Scenarios

- Rust, Swift, and JavaScript workflow scaffolds bind each effect protocol to a declared executor role and exact callable symbol.
- Swift executors preserve their declared request and result types across synchronous, `async`, `throws`, `rethrows`, and typed-throws declarations.
- A protocol with a missing symbol, wrong role, missing function, incompatible request/result signature, or executor-owned transition is diagnosed deterministically.
- A one-request-one-result executor containing a lifecycle loop is rejected.

## Commands

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml workflow_rust_scaffold_generates_traceable_orchestration_roles
cargo test --manifest-path tooling/rust/rms/Cargo.toml effectful_swift_and_js_scaffolds_bind_machine_drivers_and_exact_executors
cargo test --manifest-path tooling/rust/rms/Cargo.toml swift_effect_executor_conformance_accepts_async_throwing_declarations
cargo test --manifest-path tooling/rust/rms/Cargo.toml swift_signature_parser_preserves_types_across_execution_modifiers
cargo test --manifest-path tooling/rust/rms/Cargo.toml swift_signature_parser_does_not_hide_wrong_executor_types
cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_boundary_rejects_hidden_imperative_effect_loop_bypassing_machine_driver
```

## Expected Result

Generated implementations and valid binding-native Swift execution modifiers pass structure checks with an exact executor for every protocol. Modifiers do not erase the parsed request or result type. Invalid fixtures report the corresponding `structure.effect-protocol-executor-*` or `structure.effectful-control-flow-outside-machine-driver` diagnostic, and strict audit treats it as blocking.

Source provenance: the committed candidate revision containing this evidence and the named tests is resolved by `rms audit --root . --strict`.

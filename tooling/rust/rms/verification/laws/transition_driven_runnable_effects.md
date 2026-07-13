# Transition-Driven Runnable Effects

Promise: `runnable-effects-are-transition-driven`

## Scenarios

- An effectful stateful scaffold declares one exact machine driver in a `machine_driver` role.
- The driver accepts state plus classified input, calls the canonical transition, invokes only declared exact executors, and feeds typed results back through the transition.
- An effect-emitting runnable surface that calls an adapter-owned process loop instead of the driver is rejected even when a decorative machine and valid executor also exist.

## Commands

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml workflow_rust_scaffold_generates_traceable_orchestration_roles
cargo test --manifest-path tooling/rust/rms/Cargo.toml effectful_swift_and_js_scaffolds_bind_machine_drivers_and_exact_executors
cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_boundary_rejects_hidden_imperative_effect_loop_bypassing_machine_driver
```

## Expected Result

Generated effectful stateful bindings expose the complete surface-to-driver-to-transition-to-executor path. The hidden-loop fixture reports both `structure.runnable-surface-machine-bypass` and `structure.effectful-control-flow-outside-machine-driver`; both findings block strict audit.

Source provenance: the committed candidate revision containing this evidence and the named tests is resolved by `rms audit --root . --strict`.

# Property Evidence: runnable-machine-reachability

Promise:

- Property `runnable-machine-reachability` proves `runnable-surfaces-execute-declared-machines`.

Input space:

```yaml
fixtures: runnable delegates that reach or bypass their declared transition or driver
```

Oracle:

- live surface paths reach the canonical machine function
- evidence-only decorative machine paths fail

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml inspectable_boundary_io_requires_a_protocol_and_exact_surface_callable`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml effectful_swift_and_js_scaffolds_bind_machine_drivers_and_exact_executors`

Observed result:

- File-only runnable delegation is rejected; generated inspectable effectful bindings retain exact driver and executor callables.

Source revision: recorded by git commit or strict audit provenance before production use.

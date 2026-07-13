# Property Evidence: effectful-driver-preserves-transition-records

Promise:

- Property `effectful-driver-preserves-transition-records` proves `runnable-effects-are-transition-driven`.

Input space:

```yaml
fixtures:
- driver records transition output only
- driver records full transition record with state input branch and provenance
```

Oracle:

- effectful drivers invoke the declared transition-record function
- live driver history stores complete transition records rather than outputs alone

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml js_binding_rejects_output_only_effectful_machine_driver`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml scaffold_workflow_rust_generates_traceable_inner_structure`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml scaffold_workflow_swift_and_js_generate_traceable_inner_structure`

Expected result:

- An output-only driver reports `structure.machine-driver-transition-record-not-preserved`.
- Conforming Rust, Swift, and JavaScript drivers call the declared record function, retain complete records, advance from `state_after`, and execute only `output.effects`.
- Any future failing fixture is recorded under `verification/fuzz/counterexamples/effectful-driver-records` with `spec: rms/property-counterexample/v0.1`.

Source revision: recorded by git commit or strict audit provenance before production use.

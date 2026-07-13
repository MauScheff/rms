# Property Evidence: message-envelope-binding-conformance

Promise:

- Property `message-envelope-binding-conformance` proves `declared-message-envelopes-are-binding-realized`.

Input space:

```yaml
fixtures:
- declared envelope absent from representation source
- declared envelope present as a binding-native type or constructor
```

Oracle:

- each declared envelope is represented in its binding
- undeclared no-effect envelopes are not generated

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml js_binding_rejects_declared_message_envelope_without_representation`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml scaffold_workflow_rust_generates_traceable_inner_structure`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml scaffold_workflow_swift_and_js_generate_traceable_inner_structure`

Expected result:

- A missing declared envelope reports `structure.declared-message-envelope-not-represented`.
- Generated inspectable bindings represent every applicable declared envelope and omit effect envelopes when no effects exist.
- Any future failing fixture is recorded under `verification/fuzz/counterexamples/message-envelope-binding` with `spec: rms/property-counterexample/v0.1`.

Source revision: recorded by git commit or strict audit provenance before production use.

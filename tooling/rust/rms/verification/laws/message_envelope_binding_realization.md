# Law Evidence: declared message envelopes are binding-realized

Promise:

- `declared-message-envelopes-are-binding-realized`: every message envelope named by canonical machine semantics exists as a binding-native representation; no-effect machines do not acquire undeclared effect envelopes.

Scenario:

- A JavaScript fixture declares a command envelope but omits its constructor from the representation role.
- Generated Rust, Swift, and JavaScript workflow fixtures declare and realize command, event, effect, and effect-result envelopes.
- Generated stateless no-effect fixtures omit effect and effect-result envelopes from both semantics and source.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml js_binding_rejects_declared_message_envelope_without_representation`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml scaffold_workflow_rust_generates_traceable_inner_structure`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml scaffold_workflow_swift_and_js_generate_traceable_inner_structure`

Expected result:

- The malformed fixture reports `structure.declared-message-envelope-not-represented`.
- Inspectable generated bindings contain every applicable declared envelope and pass their binding verification.
- No-effect scaffolds contain no effect-envelope residue.

Source revision: resolved from the candidate Git commit by strict audit.

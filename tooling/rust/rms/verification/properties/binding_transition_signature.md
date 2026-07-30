# Property Evidence: canonical transition binding

Promise:

- `binding-transition-signature-realizes-canonical-types`

Scenarios:

- Rust and Swift stateful transitions with the right arity but unrelated parameter and return types are rejected.
- JavaScript stateful transitions whose parameters do not identify state and closed machine input roles are rejected.
- Python transitions whose annotations do not identify the canonical command or
  transition output are rejected.
- An effectful adapter cannot be declared as the canonical pure transition or stand in for the transition role.
- `rms machine apply` creates or repairs the pure transition semantic-function owner.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_typing_rejects_same_arity_noncanonical_transition_signature`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml swift_typing_rejects_same_arity_noncanonical_transition_signature`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml js_stateful_transition_rejects_noncanonical_parameter_roles`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml python_static_inspection_rejects_malformed_and_unsafe_sources`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml structure_rejects_effectful_adapter_as_canonical_transition`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml machine_apply_change_yaml_updates_canonical_machine_structure`

Expected result:

- All tests pass.
- Inspectable stateful bindings expose the canonical state-plus-input path and transition-shaped output.
- Canonical transitions are pure semantic functions defined in declared transition roles; effects remain in effectful roles.

Source revision: resolved by strict audit from the committed candidate.

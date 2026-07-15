# Evidence: public behavior bindings close contract-to-machine paths

Promise:

- public-behavior-bindings-close-contract-to-machine-path

Scenario:

- Validate one implemented public command without a behavior binding, then validate the same command with one exact contract, semantic-function owner, machine input, and machine output binding.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml semantic_graph::tests::implemented_public_command_requires_an_exact_behavior_binding -- --exact`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml semantic_graph::tests::exact_public_binding_closes_contract_to_machine_path -- --exact`
- `cargo run --manifest-path tooling/rust/rms/Cargo.toml -- validate --root . --json`

Expected result:

- The unbound fixture emits `semantic.public-binding-missing`.
- The bound fixture emits `bound-through`, `delegates-to`, and `maps-to` edges and no missing or unreachable public behavior diagnostic.
- Repository validation rejects stale or unresolved public and dependency bindings.

Source revision: resolved from the candidate Git commit by `rms audit --root . --strict`.

# Contract Evidence: apply-semantic-change

Promise: `apply-semantic-change` validates a complete `rms/semantic-change/v0.1`, updates laws, explicitly typed provided or required contracts, behavior and semantic-function bindings, machine declarations, runnable surfaces, properties, and evidence obligations together, records the exact change, and seals the resulting canonical semantic revision without changing module topology.

Scenario: valid YAML adds a law, public contract, machine transition, implementation semantic function, and evidence files; required-contract set changes remain consumer-owned; a law without evidence, a function with an unknown promise, ambiguous contract ownership, accidental dependency publication, removal of the only invariant authority owner, or ambiguous `surfaces.set: []` is rejected without mutation.

Commands:

```sh
cargo test --workspace --locked spec_apply_change_yaml_updates_semantics_and_machine_structure
cargo test --workspace --locked spec_apply_contract_set_replaces_scaffold_without_duplicate_surface
cargo test --workspace --locked spec_apply_contract_remove_deletes_surface_and_file
cargo test --workspace --locked spec_apply_contract_set_infers_required_ownership_without_publishing_command
cargo test --workspace --locked spec_apply_contract_add_does_not_implicitly_publish_required_contract
cargo test --workspace --locked spec_apply_contract_add_can_declare_required_consumer_expectation
cargo test --workspace --locked spec_apply_contract_set_requires_explicit_kind
cargo test --workspace --locked spec_apply_publishes_first_capability_on_standalone_module
cargo test --workspace --locked spec_apply_required_contract_remove_preserves_shared_provider_artifact
cargo test --workspace --locked spec_apply_rejects_law_without_evidence
cargo test --workspace --locked spec_apply_gates_semantic_function_add_set_and_remove
cargo test --workspace --locked spec_apply_rejects_ambiguous_empty_surface_replacement
cargo test --workspace --locked semantic_change_treats_noop_machine_section_as_empty
cargo test --workspace --locked semantic_revision_detects_direct_canonical_manifest_drift
cargo test --workspace --locked semantic_change_record_names_are_bounded
cargo test --workspace --locked semantic_apply_rolls_back_when_change_record_cannot_be_written
```

Expected result: semantic changes update canonical artifacts and records together or restore the complete pre-apply filesystem state; derived record names remain within portable component limits; dry-run exposes the final model; empty machine sections remain inert; contract kind, direction, capability publication, and behavior bindings remain atomic; a standalone module can publish its first capability without generating directories; shared artifacts are retained while referenced; authority gaps are rejected; and direct manifest edits after apply invalidate semantic revision integrity.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

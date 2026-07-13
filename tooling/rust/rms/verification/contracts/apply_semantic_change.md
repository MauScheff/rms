# Contract Evidence: apply-semantic-change

Promise: `apply-semantic-change` validates a complete `rms/semantic-change/v0.1`, updates laws, ownership-aware provided or required contracts, semantic-function bindings, machine declarations, runnable surfaces, properties, and evidence obligations together, records the exact change, and seals the resulting canonical semantic revision.

Scenario: valid YAML adds a law, public contract, machine transition, implementation semantic function, and evidence files; required-contract set changes remain consumer-owned; a law without evidence, a function with an unknown promise, ambiguous contract ownership, accidental dependency publication, removal of the only invariant authority owner, or ambiguous `surfaces.set: []` is rejected without mutation.

Commands:

```sh
cargo test --workspace --locked spec_apply_change_yaml_updates_semantics_and_machine_structure
cargo test --workspace --locked spec_apply_contract_set_replaces_scaffold_without_duplicate_surface
cargo test --workspace --locked spec_apply_contract_remove_deletes_surface_and_file
cargo test --workspace --locked spec_apply_contract_set_infers_required_ownership_without_publishing_command
cargo test --workspace --locked spec_apply_contract_add_does_not_implicitly_publish_required_contract
cargo test --workspace --locked spec_apply_contract_add_can_declare_required_consumer_expectation
cargo test --workspace --locked spec_apply_contract_set_requires_direction_when_name_is_provided_and_required
cargo test --workspace --locked spec_apply_required_contract_remove_preserves_shared_provider_artifact
cargo test --workspace --locked spec_apply_rejects_law_without_evidence
cargo test --workspace --locked spec_apply_gates_semantic_function_add_set_and_remove
cargo test --workspace --locked spec_apply_rejects_ambiguous_empty_surface_replacement
cargo test --workspace --locked semantic_change_rejects_noop_machine_section
cargo test --workspace --locked semantic_revision_detects_direct_canonical_manifest_drift
```

Expected result: semantic changes update canonical artifacts and records together; dry-run exposes the final machine and semantic-function set; no-op machine sections fail; contract and function revisions remain atomic; provided and required contract ownership is preserved; shared contract artifacts are not deleted while referenced; existing runnable surfaces survive unrelated changes; authority gaps are rejected; and direct manifest edits after apply invalidate semantic revision integrity.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

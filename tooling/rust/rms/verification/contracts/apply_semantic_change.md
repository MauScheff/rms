# Contract Evidence: apply-semantic-change

Promise: `apply-semantic-change` validates a complete `rms/semantic-change/v0.1`, updates laws, contracts, machine declarations, runnable surfaces, properties, and evidence obligations together, records the exact change, and seals the resulting canonical semantic revision.

Scenario: valid YAML adds a law, public contract, machine transition, and evidence files; a law without evidence is rejected.

Commands:

```sh
cargo test --workspace --locked spec_apply_change_yaml_updates_semantics_and_machine_structure
cargo test --workspace --locked spec_apply_contract_set_replaces_scaffold_without_duplicate_surface
cargo test --workspace --locked spec_apply_contract_remove_deletes_surface_and_file
cargo test --workspace --locked spec_apply_rejects_law_without_evidence
cargo test --workspace --locked semantic_change_rejects_noop_machine_section
cargo test --workspace --locked semantic_revision_detects_direct_canonical_manifest_drift
```

Expected result: semantic changes update canonical artifacts and records together; dry-run exposes the final machine; no-op machine sections fail; contract revisions remain atomic; and direct manifest edits after apply invalidate semantic revision integrity.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

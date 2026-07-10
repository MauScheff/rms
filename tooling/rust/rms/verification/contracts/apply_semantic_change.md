# Contract Evidence: apply-semantic-change

Promise: `apply-semantic-change` validates `rms/semantic-change/v0.1` and updates laws, contracts, machine declarations, runnable surface declarations, and evidence obligations together. Existing contracts can be replaced or removed without hand-editing canonical files.

Scenario: valid YAML adds a law, public contract, machine transition, and evidence files; a law without evidence is rejected.

Commands:

```sh
cargo test --workspace --locked spec_apply_change_yaml_updates_semantics_and_machine_structure
cargo test --workspace --locked spec_apply_contract_set_replaces_scaffold_without_duplicate_surface
cargo test --workspace --locked spec_apply_contract_remove_deletes_surface_and_file
cargo test --workspace --locked spec_apply_rejects_law_without_evidence
```

Expected result: semantic changes update their canonical artifacts and change records together; `contracts.set` replaces one existing command contract, `contracts.remove` removes its command and conventional file, and invalid semantic deltas fail before writes.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

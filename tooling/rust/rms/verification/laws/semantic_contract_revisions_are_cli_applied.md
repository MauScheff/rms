# Semantic Contract Revisions Are CLI Applied

Promise: `rms spec apply` can replace, add, or remove provided and required capability contracts while preserving ownership, canonical references, shared files, and the committed semantic-change record. RMS self-development is the single explicit exception: repository maintainers may seal its native changes independently because the candidate tool cannot confer authority on its own implementation.

Scenarios: replace a generated provider contract without duplicating its command; revise a required capability contract without publishing it; reject an implicit add that would transfer requirement ownership; require direction when one name is both provided and required; preserve a shared contract file when only one direction is removed.

Commands:

```sh
cargo test --workspace --locked spec_apply_contract_set_replaces_scaffold_without_duplicate_surface
cargo test --workspace --locked spec_apply_contract_remove_deletes_surface_and_file
cargo test --workspace --locked spec_apply_contract_set_infers_required_ownership_without_publishing_command
cargo test --workspace --locked spec_apply_contract_add_does_not_implicitly_publish_required_contract
cargo test --workspace --locked spec_apply_contract_add_can_declare_required_consumer_expectation
cargo test --workspace --locked spec_apply_contract_set_requires_direction_when_name_is_provided_and_required
cargo test --workspace --locked spec_apply_required_contract_remove_preserves_shared_provider_artifact
```

Expected result: `direction: provided|required` selects the correct ownership lane; unambiguous set/remove operations are normalized to an explicit direction in the sealed record; consumer revisions never publish provider commands; ambiguous or ownership-transferring operations fail before mutation; and a contract file remains while any canonical reference still uses it. The independent maintainer seal is accepted only for the canonical `rms-cli` module with its explicit self-application declaration, evidence, and public ownership prerequisites.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

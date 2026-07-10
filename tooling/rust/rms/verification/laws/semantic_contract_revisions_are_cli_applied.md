# Semantic Contract Revisions Are CLI Applied

Promise: `rms spec apply` can replace or remove an existing public contract while updating its contract file, module public surface, and committed semantic-change record together.

Scenarios: replace a generated contract without duplicating its command; remove a contract and its command intentionally.

Commands:

```sh
cargo test --workspace --locked spec_apply_contract_set_replaces_scaffold_without_duplicate_surface
cargo test --workspace --locked spec_apply_contract_remove_deletes_surface_and_file
```

Expected result: `contracts.set` rewrites the canonical file and preserves one public command, while `contracts.remove` removes both the command declaration and conventional contract file.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

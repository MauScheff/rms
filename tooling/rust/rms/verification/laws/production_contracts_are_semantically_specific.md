# Production Contracts Are Semantically Specific

Promise: a generated capability contract remains visibly incomplete until a semantic change replaces its generic meaning, assumptions, outcomes, and failures.

Scenario: validate a generated contract containing `x-rms.scaffold: true` and the generic capability statements, then replace it through `contracts.set` with product-specific semantics.

Commands:

```sh
cargo test --workspace --locked generated_contract_semantics_are_visible_scaffold_obligations
cargo test --workspace --locked spec_apply_contract_set_replaces_scaffold_without_duplicate_surface
```

Expected result: the scaffold fixture reports `semantic.contract-scaffold-active`; the revised contract has one public command, contains its declared meaning and guarantees, removes the scaffold marker, and produces no scaffold diagnostic.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

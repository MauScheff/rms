# Property Evidence: runnable delegation owner

Promise: `runnable-delegation-has-semantic-owner`.

Input space: runnable surface declarations using valid and unknown role names, concrete symbols, declared effects, and effectless exceptions.

Oracle:

- role delegation resolves through `architecture.roles` or a concrete symbol is named;
- each runnable surface declares boundary effects or a nonempty no-effect justification;
- missing ownership and effect policy are strict surface blockers.

Command/tool: `cargo test --workspace --locked runnable_surface_requires_declared_delegate_role_and_effect_policy`.

Expected result: unknown role delegation and an omitted effect policy produce focused deterministic diagnostics.

Source provenance: the clean committed candidate revision resolved by strict audit.

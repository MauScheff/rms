# Law Evidence: composite proof delegation

Promise: invariant `composite-laws-may-delegate-proof-through-exports` permits delegation only through a contained provider, provider law/property, public export, and concrete evidence.

Command/tool: `cargo test --manifest-path Cargo.toml composite_proof_delegation_resolves_child_law_property_and_export`

Expected and observed result: the complete delegation composes; changing the provider property makes composition incompatible. The focused fixture passes.

Source revision: resolved from the committed candidate by strict audit.

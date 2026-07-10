# Property Evidence: public domain validity

Promise: `public-domain-values-preserve-validity`.

Input space: Rust and Swift public structs representing domain values, plus declared transport envelopes, transition outputs, transition records, and source-provenance records.

Oracle:

- domain structs with exposed fields fail binding conformance;
- adding a domain struct to `allowed_public_field_structs` does not silence the failure;
- declared structural records retain their idiomatic public-field representation.

Command/tool: `cargo test --workspace --locked public_domain_field_allowlist_bypass`.

Expected result: Rust and Swift bypass attempts fail with `structure.public-domain-field-bypass`.

Source provenance: the clean committed candidate revision resolved by strict audit.

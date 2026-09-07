# Contract Evidence: add-external-rust-crate

Promise:

- A ready route receipt authorizes one external Rust crate declaration for one implementation v0.2 binding.
- The declaration records the Rust import identity, Cargo package identity, exact version-requirement string, and matching allowlist entry.
- The mutator preserves `dependencies.local_modules` and does not edit `Cargo.toml`.
- Rust validation rejects a Cargo package or version requirement that contradicts the canonical declaration.

Command/tool:

- `cargo test -p rms external_rust_crate -- --nocapture`
- `cargo test -p rms next_routes_external_rust_crate_declaration_to_bounded_mutator -- --nocapture`

Expected result:

- The route is an implementation-candidate route whose receipt permits `external-crate-add` and rejects unrelated canonical mutation.
- An identical declaration is idempotent. A conflicting redeclaration fails.
- A renamed Cargo dependency passes only when its dependency key, package, and version requirement match the canonical declaration.
- Cargo and RMS-local module dependencies remain byte-for-byte unchanged by the declaration mutator.

Source revision: recorded by git commit or strict audit provenance before production use.

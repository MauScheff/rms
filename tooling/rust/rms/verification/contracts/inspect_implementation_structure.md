# Contract Evidence: inspect-implementation-structure

Proves: `check-machine-structure`.

Covered by `cargo test --manifest-path tooling/rust/rms/Cargo.toml`, including scaffold tests that build structure reports from generated Rust, Swift, JavaScript, and Python bindings and assert domain-named machine/role declarations, represented message envelopes, exact transition-record functions, complete live driver records, trace roles, replay support, and first-bad-transition support.

Diagnostic tests verify absent declared envelopes, output-only live drivers, unchecked transition arithmetic, missing transition output, unnamed transition cases, workflow trace gaps, projection command emission, parser role aliases, cross-module private-role imports, undeclared effect/effect-result residue, undeclared runnable delegation, and missing declared symbols produce focused diagnostics. Rust and Swift signature tests reject same-arity transitions using unrelated binding types, JavaScript checks reject parameters that do not realize state/input roles, and transition ownership checks reject effectful adapters as canonical pure transitions. `rust_typing_rejects_public_domain_field_allowlist_bypass` and the corresponding Swift check keep domain values behind validated construction while allowing declared envelopes and transition/provenance records.

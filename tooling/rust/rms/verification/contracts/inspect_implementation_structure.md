# Contract Evidence: inspect-implementation-structure

Proves: `check-machine-structure`.

Covered by `cargo test --manifest-path tooling/rust/rms/Cargo.toml`, including scaffold tests that build structure reports from generated Rust, Swift, and JavaScript bindings and assert domain-named machine/role declarations, message envelopes, transition output, trace roles, replay support, and first-bad-transition support.

Diagnostic tests verify missing envelopes, missing transition output, unnamed transition cases, workflow trace gaps, projection command emission, parser role aliases, unchecked numeric arithmetic, cross-module private-role imports, undeclared effect/effect-result source residue, undeclared runnable delegation, and missing declared architecture symbols produce focused structure diagnostics. `rust_typing_rejects_public_domain_field_allowlist_bypass` and the corresponding Swift check keep domain values behind validated construction while allowing declared envelopes and transition/provenance records.

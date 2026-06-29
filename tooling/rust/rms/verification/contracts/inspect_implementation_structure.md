# Contract Evidence: inspect-implementation-structure

Covered by `cargo test --manifest-path tooling/rust/rms/Cargo.toml`, including scaffold tests that build structure reports from generated Rust, Swift, and JavaScript bindings and assert domain-named machine/role declarations, message envelopes, transition output, trace roles, replay support, and first-bad-transition support.

Diagnostic tests verify missing envelopes, missing transition output, workflow trace gaps, projection command emission, and missing declared architecture symbols produce focused structure diagnostics. Repository-level `rms validate --root .` and `rms structure tooling/rust/rms/implementation.yaml` exercise the command against canonical artifacts and confirm findings remain derived diagnostics.

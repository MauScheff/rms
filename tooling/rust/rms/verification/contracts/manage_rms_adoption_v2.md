# Contract evidence: manage-rms-adoption v2

Progressive checks state the bounded certification first, enumerate every discovered closure, and report live total and changed unowned production paths without treating unrelated code as certified.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml progressive_coverage_names_every_closure`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml adoption_complete_rejects_unowned_production_paths`

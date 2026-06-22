# Contract Evidence: add-rms-module

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic scaffold coverage.

Executable coverage:

- `rust_module_scaffold_generates_valid_binding_artifacts` verifies generated Rust `module.yaml` and `implementation.yaml` validate, the module README records the requested purpose and Rust binding, and generated guidance routes future work through public contracts, declared verification, constructor-backed domain values, and query-produced read-model exceptions.
- `swift_module_scaffold_generates_valid_binding_artifacts` verifies the same scaffold guarantees for the Swift binding.
- `rms release check --root .` runs a scaffold roundtrip that initializes a new RMS system, adds Rust and Swift modules, validates and composes the scaffold, and verifies the generated Rust binding.

Compatibility:

- This is additive within RMS 0.1. Existing `rms add-module` invocations remain valid.
- New generated guidance files are written only in a fresh module directory; the command still refuses to overwrite existing generated files.

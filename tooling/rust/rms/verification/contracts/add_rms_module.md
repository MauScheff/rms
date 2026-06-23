# Contract Evidence: add-rms-module

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic scaffold coverage.

Executable coverage:

- `rust_module_scaffold_generates_valid_binding_artifacts` verifies generated Rust `module.yaml` and `implementation.yaml` validate, the module README records the requested purpose and Rust binding, and generated guidance routes future work through public contracts, declared verification, constructor-backed domain values, and query-produced read-model exceptions.
- `swift_module_scaffold_generates_valid_binding_artifacts` verifies the same scaffold guarantees for the Swift binding.
- `executable_module_scaffold_generates_valid_binding_artifacts` verifies the executable binding scaffold validates with the Boundary profile, declares opaque command-backed verification semantics, writes build and smoke scripts, records boundary evidence, and passes `rms verify implementation.yaml`.
- `module_scaffold_generates_required_profile_sections` verifies requested Stateful, Distributed, Workflow, and Boundary profiles produce the required empty profile sections while keeping module-specific semantics unset.
- `rms release check --root .` runs a scaffold roundtrip that initializes a new RMS system, adds Rust, Swift, and Boundary-profile executable modules, validates and composes the scaffold, verifies the generated executable binding, and verifies the generated Rust binding.

Compatibility:

- This is additive within RMS 0.1. Existing `rms add-module` invocations remain valid.
- New generated guidance files are written only in a fresh module directory; the command still refuses to overwrite existing generated files.

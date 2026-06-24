# Contract Evidence: add-rms-module

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic scaffold coverage.

Executable coverage:

- `rust_module_scaffold_generates_valid_binding_artifacts` verifies generated Rust `module.yaml` and `implementation.yaml` validate, the module README records semantic shape guidance, and generated code separates representation and transition roles.
- `swift_module_scaffold_generates_valid_binding_artifacts` verifies the same scaffold guarantees for the Swift binding.
- `js_boundary_adapter_scaffold_separates_representation_parser_and_adapters` verifies the JS binding separates representation, parser, ports, and adapter roles and passes `rms verify implementation.yaml`.
- `executable_module_scaffold_generates_valid_binding_artifacts` verifies the executable binding scaffold validates with the Boundary profile, declares opaque command-backed verification semantics, writes build and smoke scripts, records boundary evidence, and passes `rms verify implementation.yaml` without claiming static insight into opaque executable internals.
- `module_scaffold_generates_required_profile_sections` verifies requested Stateful, Distributed, Workflow, and Boundary profiles produce the required empty profile sections while keeping module-specific semantics unset.
- `composite_module_scaffold_generates_composition_section` verifies `--shape composite` records `kind: composite` and an explicit empty `composition` block.
- `add_capability_scaffolds_recursive_tree_that_verifies` verifies recursive capability scaffolds include parent export, domain transition, accepted/rejected, malformed input, and parser-to-domain-command evidence placeholders.
- `rms release check --root .` runs a scaffold roundtrip that initializes a new RMS system, adds Rust, Swift, and Boundary-profile executable modules, validates and composes the scaffold, verifies the generated executable binding, and verifies the generated Rust binding.

Compatibility:

- This is additive within RMS 0.1. Existing `rms add-module` invocations remain valid.
- New generated guidance files are written only in a fresh module directory; the command still refuses to overwrite existing generated files.
- Provider scaffold plans are advisory run records only; canonical meaning remains in generated manifests, contracts, implementation bindings, and evidence files.

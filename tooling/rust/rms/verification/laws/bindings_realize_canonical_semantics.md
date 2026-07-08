# Law Evidence: bindings realize canonical semantics

Promise:

- Binding adapters realize canonical RMS semantics: module shape, machine roles, runnable surfaces, evidence obligations, public contracts, and dependency links come from canonical manifests and changes.
- Rust, Swift, JS, and executable bindings render or inspect those semantics idiomatically, but do not invent semantic roles independently.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_module_scaffold_generates_valid_binding_artifacts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml swift_module_scaffold_generates_valid_binding_artifacts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml js_boundary_adapter_scaffold_separates_representation_parser_and_adapters`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml executable_module_scaffold_generates_valid_binding_artifacts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml add_capability_scaffolds_recursive_tree_that_verifies`

Expected result:

- All inspectable binding scaffolds derive their machine variants, roles, trace bundles, and verification commands from one canonical scaffold model before rendering binding-specific files.
- Generated implementation manifests validate, verify, and structure-check without binding-specific semantic drift.

Source revision: recorded by git commit before production release.

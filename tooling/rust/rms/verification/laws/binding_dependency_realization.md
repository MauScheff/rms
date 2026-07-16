# Law Evidence: binding dependency realization

Promise:

- `binding-dependencies-are-adapter-realized`: canonical local RMS module identities are translated by binding adapters into the metadata needed by each implementation ecosystem.

Scenario:

- `rms add-capability-tree` with Rust domain and boundary children records `dependencies.local_modules`, adds one Rust allowlist entry, and writes the sibling Cargo path dependency.
- `rms spec apply` accepts language-neutral `binding_dependencies.set/add/remove` and routes realization through the selected binding adapter.
- Rust package spelling `mini-xargs-domain` and import spelling `mini_xargs_domain` normalize to the same crate identity.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml add_capability_realizes_same_binding_local_dependency`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml spec_apply_realizes_language_neutral_binding_dependencies`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_dependency_names_normalize_package_and_import_spellings`

Expected result:

- Generated same-binding capability children compose without hand-editing implementation allowlists or native Rust dependency metadata.
- The semantic change record contains RMS module ids rather than language-specific package spellings.

Source revision: resolved from the candidate Git commit by strict audit.

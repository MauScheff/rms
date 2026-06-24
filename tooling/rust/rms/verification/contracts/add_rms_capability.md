# Contract Evidence: add-rms-capability

Covered by `cargo test --manifest-path Cargo.toml`.

Executable coverage:

- `add_capability_scaffolds_recursive_tree_that_verifies` initializes a fresh RMS system, runs `rms add-capability` with a Rust domain child and JS boundary child, validates generated manifests, checks composition, and verifies the composite parent rollup.

Compatibility:

- This is additive within RMS 0.1. Existing `rms add-module` scaffolds remain valid.
- Generated artifacts are written only through `write_new_file`, so existing files are preserved.

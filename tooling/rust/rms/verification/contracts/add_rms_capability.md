# Contract Evidence: add-rms-capability

Covered by `cargo test --manifest-path Cargo.toml`.

Executable coverage:

- `add_capability_scaffolds_recursive_tree_that_verifies` initializes a fresh RMS system, runs `rms add-capability` with a Rust domain child and JS boundary child, validates generated manifests, checks composition, verifies generated trace bundles, and verifies the composite parent rollup.
- The same test asserts generated inner names use semantic capability language, such as `PlayGameMachine` and `PlayGameBoundaryMachine`, rather than leaking child role suffixes such as `RulesMachine`.
- `add_capability_browser_tool_declares_thin_runnable_surface` verifies local browser/tool intent creates a thin stateless boundary surface with declared controller entrypoint `public/controller.mjs`, launch script `public/app.mjs`, launch host `public/index.html`, and no copied domain logic in the surface.

Compatibility:

- This is additive within RMS 0.1. Existing `rms add-module` scaffolds remain valid.
- Generated artifacts are written only through `write_new_file`, so existing files are preserved.

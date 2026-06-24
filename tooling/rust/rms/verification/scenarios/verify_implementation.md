# Scenario Evidence: verify-implementation

`rms verify <implementation.yaml>` runs the command declared by the implementation binding from the binding directory and reports failure when the native command fails.

`rms verify <module.yaml>` accepts composite module manifests. For composites it validates the parent manifest, runs repository composition, requires parent scenario evidence, and runs `rms verify` for each contained child with an `implementation.yaml`.

Covered by `cargo test --manifest-path Cargo.toml`, including `composite_module_verify_rolls_up_child_implementations`.

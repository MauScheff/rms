# Scenario Evidence: verify-implementation

`rms verify <implementation.yaml>` runs the command declared by the implementation binding from the binding directory and reports failure when the native command fails.

After the native command passes, `rms verify <implementation.yaml>` checks declared local trace bundles from implementation roles, semantic-function `evidence.traces`, and `verification/traces`. A structurally bad trace bundle fails verification even when the native command succeeds.

`rms verify <module.yaml>` accepts composite module manifests. For composites it validates the parent manifest, runs repository composition, requires parent scenario evidence, and runs `rms verify` for each contained child with an `implementation.yaml`.

Covered by `cargo test --manifest-path Cargo.toml`, including `composite_module_verify_rolls_up_child_implementations` and `verify_fails_bad_declared_trace_bundle`.

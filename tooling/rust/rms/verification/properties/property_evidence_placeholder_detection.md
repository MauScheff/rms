# Property Evidence: property evidence placeholder detection

Promise:

- Property `property-evidence-placeholder-detection` proves `property-evidence-obligation-is-not-proof`.

Input space:

- Property evidence produced directly by `rms spec apply`.
- Concrete property evidence naming an executed command and observed result.

Oracle:

- Untouched generated evidence reports `evidence.placeholder`.
- Concrete observed evidence does not report that diagnostic.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml spec_apply_change_yaml_adds_semantic_property_and_evidence`
- `cargo run --manifest-path tooling/rust/rms/Cargo.toml -- validate --root .`

Observed result:

- The focused regression passed with one generated obligation classified as `evidence.placeholder`.
- Root validation no longer reports the diagnostic after the obligation files are replaced by this observed evidence.
- A failing generated case remains recordable under `verification/fuzz/counterexamples/property-evidence`.

Source revision: resolved and recorded by `rms audit --strict` for the committed release candidate.

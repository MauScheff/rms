# Law Evidence: non-corpus property realizations are code-backed

Promise:

- Law `non-corpus-property-realization-is-code-backed` requires generated, exhaustive, coverage-guided, and model-checking claims to name exact binding generators and runners.

Scenario:

- Check generated-property realizations with a missing generator, missing runner, or unresolved runner symbol.
- Check a runner that calls an algorithmic generator, the declared operation, and an assertion oracle.
- Check a fixed literal collection mislabeled as generated-property.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml property_realizations_require_generators_runners_operations_and_oracles`

Observed result:

- The regression test passed.
- RMS emitted distinct generator, runner, operation, and oracle diagnostics for incomplete realizations.
- The complete runner over `src/property.rs#generate_values` passed, while the fixed generator was classified as a deterministic corpus.

Source revision: resolved and recorded by `rms audit --strict` for the committed release candidate.

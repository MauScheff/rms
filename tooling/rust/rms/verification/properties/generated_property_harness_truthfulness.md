# Property Evidence: generated-property-harness-truthfulness

Promise:

- Property `generated-property-harness-truthfulness` proves `generated-property-claims-use-generators`.

Input space:

```yaml
fixtures: generated harnesses and fixed literal case providers across inspectable bindings
```

Oracle:

- generator-backed harnesses pass
- fixed literal arrays labeled generated-property fail

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml property_realizations_require_existing_non_corpus_harness_symbols`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml scaffold_generates`

Observed result:

- Fixed Rust array/vector harnesses are rejected as corpora; binding fixtures use algorithmic map-based case construction and pass property conformance.

Source revision: recorded by git commit or strict audit provenance before production use.

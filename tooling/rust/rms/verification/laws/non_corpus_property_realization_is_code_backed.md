# Law Evidence: non-corpus property realizations are code-backed

Promise:

- Law `non-corpus-property-realization-is-code-backed` requires generated, exhaustive, coverage-guided, and model-checking claims to name an exact binding harness.

Scenario:

- Check a generated-property realization with no harness.
- Check a realization whose `path#symbol` reference names a missing source symbol.
- Check a deterministic corpus with no harness and a generated realization with an existing Rust function.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml property_realizations_require_existing_non_corpus_harness_symbols`

Observed result:

- The regression test passed.
- RMS emitted `evidence.property-realization-harness-missing` for the absent reference and `structure.property-realization-harness-symbol-missing` for the absent symbol.
- The deterministic corpus and existing `src/property.rs#generate_values` harness produced no harness diagnostic.

Source revision: resolved and recorded by `rms audit --strict` for the committed release candidate.

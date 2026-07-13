# Property Evidence: property realization harness conformance

Promise:

- Property `property-realization-harness-conformance` proves `non-corpus-property-realization-is-code-backed`.

Input space:

- Deterministic corpus realizations without harnesses.
- Generated-property realizations with absent, missing, and existing Rust `path#symbol` references.
- Scaffolded Rust, Swift, and JavaScript generated-case harnesses.

Oracle:

- Corpus claims may omit a harness.
- Every stronger claim must name a safe relative `path#symbol` reference.
- Inspectable bindings must contain the named function or tagged callable.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml property_realizations_require_existing_non_corpus_harness_symbols`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_module_scaffold_generates_valid_binding_artifacts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml swift_module_scaffold_generates_valid_binding_artifacts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml js_boundary_adapter_scaffold_separates_representation_parser_and_adapters`

Observed result:

- Missing references and missing symbols were rejected by distinct deterministic diagnostics.
- The concrete Rust harness passed.
- Generated binding scaffolds declare and exercise `generate_property_cases` or `generate_malformed_input_cases` through their native verification commands.
- Counterexamples remain replayable under `verification/fuzz/counterexamples/property-harness`.

Source revision: resolved and recorded by `rms audit --strict` for the committed release candidate.

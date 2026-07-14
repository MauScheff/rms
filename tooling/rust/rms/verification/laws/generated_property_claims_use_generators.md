# Law Evidence: generated-property claims use generators

Promise:

- A generated-property realization constructs cases from its declared space; fixed literal arrays and vectors are classified as deterministic corpora.

Scenario:

- A Rust realization points at `generate_values`, which returns `vec![0, 1, 2]` while claiming `generated-property`.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml property_realizations_require_generators_runners_operations_and_oracles`

Expected result:

- RMS reports `evidence.property-realization-fixed-corpus`; missing generators, runners, operations, and oracles remain separately diagnosed.

Source revision: recorded by git commit or strict audit provenance before production use.

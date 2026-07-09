# Contract Evidence: check-semantic-properties

Promise:

- `check-semantic-properties` exposes `rms property check` for module and implementation targets.
- The command reports semantic property and fuzz targets, missing input spaces, missing oracles, missing concrete evidence, and unreplayable counterexamples.

Command/tool:

- `cargo run --manifest-path tooling/rust/rms/Cargo.toml -- property check tooling/rust/rms/module.yaml`
- `cargo run --manifest-path tooling/rust/rms/Cargo.toml -- property check tooling/rust/rms/module.yaml --strict`

Expected result:

- non-strict mode reports review-required findings without inventing semantics;
- strict mode treats property/fuzz gaps as failures for production claims;
- JSON mode uses the same report data as human output.

Source revision: recorded by git commit and strict audit provenance before production use.

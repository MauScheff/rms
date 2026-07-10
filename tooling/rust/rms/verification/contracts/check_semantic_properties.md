# Contract Evidence: check-semantic-properties

Promise:

- `check-semantic-properties` exposes `rms property check` for module and implementation targets.
- The command reports semantic property and fuzz targets, input-space openness, oracles, realization strategy, concrete evidence, and counterexample replay policy.

Command/tool:

- `cargo run --manifest-path tooling/rust/rms/Cargo.toml -- property check tooling/rust/rms/module.yaml`
- `cargo test --workspace --locked open_fuzz_claim_rejects_fixed_corpus_realization -- --nocapture`

Expected result:

- non-strict mode reports review-required findings without inventing semantics;
- strict mode treats property/fuzz gaps as failures for production claims;
- open-ended fuzz claims require a generated-property or coverage-guided realization; deterministic corpora remain valid smoke evidence but do not satisfy fuzz coverage;
- finite spaces may declare deterministic exhaustive realization;
- JSON mode uses the same report data as human output.

Source provenance: the candidate commit and these commands are recorded by strict audit before a production claim.

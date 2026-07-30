# Contract Evidence: replay-property-counterexample

Promise:

- `replay-property-counterexample` exposes `rms property replay` for `rms/property-counterexample/v0.1` files.
- A counterexample is useful evidence only when it names the property, what it proves, and either a replay command or trace reference.

Command/tool:

- `cargo test --workspace --locked property`
- `cargo run --manifest-path tooling/rust/rms/Cargo.toml -- property replay <counterexample.yaml>` in replay fixtures.

Expected result:

- replayable counterexamples pass metadata validation;
- missing property/proves/replay metadata fails before production evidence can rely on the counterexample;
- replay commands, when present, are run from the counterexample directory with `RMS_COUNTEREXAMPLE` naming the exact artifact;
- machine-readable output includes the observed replay result rather than reporting only that metadata was valid.

Source revision: recorded by git commit and strict audit provenance before production use.

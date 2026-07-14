# Property Evidence: generated-property-realization-truthfulness

Promise:

- Property `generated-property-realization-truthfulness` proves `generated-property-claims-use-generators`.

Input space:

```yaml
fixtures: generated case providers and fixed literal collections across inspectable bindings
```

Oracle:

- runners that call non-literal generators and execute an oracle pass
- fixed literal collections labeled generated-property fail

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml property_realizations_require_generators_runners_operations_and_oracles`

Observed result:

- The focused regression passed.
- The algorithmic `0..32` generator was accepted when its runner consumed all generated values, called the declared operation, and asserted the oracle.
- The fixed `[0, 1, 2]` generator was rejected with `evidence.property-realization-fixed-corpus`.
- No counterexample was produced because the accepted generated realization passed; future failures belong under `verification/fuzz/counterexamples/property-truthfulness`.

Source revision: resolved and recorded by `rms audit --strict` for the committed release candidate.

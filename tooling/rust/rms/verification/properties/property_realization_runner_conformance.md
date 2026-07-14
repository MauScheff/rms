# Property Evidence: property-realization-runner-conformance

Promise:

- Property `property-realization-runner-conformance` proves `non-corpus-property-realization-is-code-backed`.

Input space:

```yaml
description: Property realization fixtures spanning fixed corpus, exhaustive enumeration, generated cases, coverage fuzzing, and model checking.
```

Oracle:

- Every realization without an exact runner is rejected.
- Generated and exhaustive realizations without an exact generator are rejected.
- Inspectable bindings reject absent runner or generator symbols.
- A runner must call its generator when declared, execute an operation, and apply an oracle.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml property_realizations_require_generators_runners_operations_and_oracles`

Observed result:

- The focused regression passed.
- RMS accepted the realization whose runner calls its algorithmic generator, semantic operation, and assertion oracle.
- RMS independently diagnosed a missing generator, a missing runner, an unresolved runner symbol, and a fixed literal generator mislabeled as `generated-property`.
- No counterexample was produced because the valid realization passed; future failures belong under `verification/fuzz/counterexamples/property-runner` with `spec: rms/property-counterexample/v0.1`.

Source revision: resolved and recorded by `rms audit --strict` for the committed release candidate.

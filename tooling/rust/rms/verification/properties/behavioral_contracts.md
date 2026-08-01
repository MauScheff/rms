# RMS behavioral-contract conformance

Promise:

- Every tracked public contract uses `rms/contract/v0.2` and has no unresolved clause.
- Core clauses compile through the RMS property evaluator; external clauses resolve to an executable owner property.
- Invocation and transition records are judged by the same evaluator for every binding.
- Solver counterexamples are accepted only after RMS re-evaluates the model.

Evidence:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml behavioral_contract`
- `cargo test -p rms --locked behavioral_contract::tests::cvc5_reference_solver_conformance -- --ignored --exact` with the external cvc5 reference binary available
- The `behavioral-contracts` CI matrix runs the corpus, structural validation, and composition on Linux, macOS, and Windows.
- `rms check --changes --root .`

Bounded exploration and solver-unavailable results remain explicitly unresolved; neither is reported as universal success.

# RMS behavioral-contract conformance

Promise:

- Existing v0.2 contracts retain caller-obligation semantics; v0.3 contracts are total under satisfied assumptions and have no unresolved clause.
- Core clauses compile once through the shared evaluator; external clauses resolve to an executable owner property.
- Invocation and transition records are judged by the same evaluator for every binding.
- Invalid input is never accepted, typed invalid rejection has an empty frame, and assumption gaps are inconclusive without blame.
- Solver counterexamples are accepted only after RMS re-evaluates the model; invariant preservation requires paired before and after observations and does not imply an initial-state base case.
- Monitoring conclusions are bounded by `full`, `sampled`, `delayed`, `partial`, or `none` observability.

Evidence:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml behavioral_contract`
- `cargo test -p rms --locked behavioral_contract::tests::cvc5_reference_solver_conformance -- --ignored --exact` with the external cvc5 reference binary available
- The `behavioral-contracts` CI matrix runs the corpus, structural validation, and composition on Linux, macOS, and Windows.
- `rms check --changes --root .`

Bounded exploration, incomplete observability, and solver-unavailable results remain explicitly unresolved; none is reported as universal success.

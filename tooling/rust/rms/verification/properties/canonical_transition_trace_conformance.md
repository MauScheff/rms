# Property Evidence: canonical-transition-trace-conformance

Promise:

- Property `canonical-transition-trace-conformance` proves `trace-records-conform-to-canonical-transitions`.

Input space:

```yaml
fixtures: trace records with matching and mismatching case outputs
```

Oracle:

- matching input state output and rejection semantics pass
- missing events effects replies or typed rejections fail

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml trace_bundle_rejects_outputs_that_do_not_match_the_canonical_case`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml scaffold_generates`

Observed result:

- The mismatching fixture fails with `trace.canonical-transition-mismatch`; generated canonical trace fixtures pass for inspectable and executable scaffolds.

Source revision: recorded by git commit or strict audit provenance before production use.

# Property Evidence: transition-case-source-conformance

Promise:

- Property `transition-case-source-conformance` proves `transition-cases-are-code-backed`.

Input space:

```yaml
fixtures:
- declared case missing from source
- source branch missing from declarations
- unreachable lifecycle state
- trace source outside transition role
```

Oracle:

- declared and implemented cases align
- unreachable states fail
- trace provenance names declared transition source

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml structure_rejects_transition_case_drift_and_unreachable_states`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml machine_apply_rejects_unreachable_final_states_before_write`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml strict_trace_coverage_requires_code_backed_source_provenance`

Expected result:

- Every malformed fixture is rejected by its focused diagnostic; aligned source and replay provenance pass the same conformance path.

Source revision: recorded by git commit or strict audit provenance before production use.

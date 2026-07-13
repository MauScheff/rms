# Evidence: law proves transition-cases-are-code-backed

Promise:

- transition-cases-are-code-backed

Scenario:

- Check a fixture with one declared case absent from transition source, one source-only branch, and one unreachable lifecycle state.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml structure_rejects_transition_case_drift_and_unreachable_states`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml machine_apply_rejects_unreachable_final_states_before_write`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml strict_trace_coverage_requires_code_backed_source_provenance`

Expected result:

- Semantic apply rejects unreachable final states before writing. Structure diagnostics reject declaration/source drift and unreachable states. Strict trace coverage rejects provenance that does not identify the declared transition role and canonical source branch.

Source revision: recorded by git commit or strict audit provenance before production use.

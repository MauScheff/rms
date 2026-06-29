# Contract Evidence: audit-rms-project

Covered by `cargo test --manifest-path tooling/rust/rms/Cargo.toml`, including strict audit and structure diagnostic tests.

Evidence:

- `strict_audit_fails_scaffold_evidence` verifies `rms audit --strict` semantics fail scaffold placeholder evidence while non-strict audit reports review-required findings without failing the project.
- `structure_report_flags_cross_module_private_imports` verifies implementation structure diagnostics flag boundary/source role files that import another module's private `transition` or `representation` internals.
- Existing validation, composition, structure, trace, and gate tests cover the deterministic checks reused by audit reports.

The command is provider-independent. It derives production-readiness findings from RMS artifacts and local evidence, lists verification targets, and does not mutate project files.

Source revision: recorded by release or conformance tooling.

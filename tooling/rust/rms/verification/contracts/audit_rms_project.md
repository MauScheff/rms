# Contract Evidence: audit-rms-project

Covered by `cargo test --manifest-path tooling/rust/rms/Cargo.toml`, including strict audit and structure diagnostic tests.

Evidence:

- `strict_audit_fails_scaffold_evidence` verifies `rms audit --strict` semantics fail scaffold placeholder evidence while non-strict audit reports review-required findings without failing the project.
- `strict_audit_fails_without_source_revision` verifies strict audit fails when the audit root has no resolvable source revision.
- `audit_scope_excludes_examples_from_repo_root_by_default` verifies repository-root production audits skip illustrative `examples/` modules unless `--include-examples` is requested or the audit root is inside `examples`.
- `local_workspace_evidence_is_source_unpinned_without_git_revision` verifies local-workspace evidence without a git revision is reported as unpinned evidence.
- `structure_report_flags_cross_module_private_imports` verifies implementation structure diagnostics flag boundary/source role files that import another module's private `transition` or `representation` internals.
- `structure_report_flags_undeclared_effect_source_residue` verifies strict structure diagnostics flag effect/effect-result source roles that are absent from canonical machine declarations.
- Existing validation, composition, structure, trace, and gate tests cover the deterministic checks reused by audit reports.

The command is provider-independent. It derives production-readiness findings from RMS artifacts and local evidence, lists verification targets, and does not mutate project files.

Source revision: recorded by release or conformance tooling.

# Contract Evidence: audit-rms-project

Covered by `cargo test --workspace --locked`, including strict audit, expanded provenance, structure diagnostics, property realization, and trace coverage tests.

Evidence:

- `strict_audit_fails_scaffold_evidence` verifies `rms audit --strict` semantics fail scaffold placeholder evidence while non-strict audit reports review-required findings without failing the project.
- `strict_audit_fails_without_source_revision` verifies strict audit fails when the audit root has no resolvable source revision.
- `git_changed_paths_include_nested_untracked_module_files` verifies provenance and semantic-drift analysis include files beneath wholly untracked module directories.
- `audit_scope_excludes_examples_from_repo_root_by_default` verifies repository-root production audits skip illustrative `examples/` modules unless `--include-examples` is requested or the audit root is inside `examples`.
- `local_workspace_evidence_is_source_unpinned_without_git_revision` verifies local-workspace evidence without a git revision is reported as unpinned evidence.
- `generated_contract_semantics_are_visible_scaffold_obligations` verifies generated public contracts remain strict-audit blockers until their product meaning replaces scaffold semantics.
- `structure_report_flags_cross_module_private_imports` verifies implementation structure diagnostics flag boundary/source role files that import another module's private `transition` or `representation` internals.
- `structure_report_flags_undeclared_effect_source_residue` verifies strict structure diagnostics flag effect/effect-result source roles that are absent from canonical machine declarations.
- Canonical machine fixtures verify strict audit rejects collapsed semantic variants, stateful transitions without state and classified input, incomplete effect protocols, and uncovered effect-result branches.
- Property fixtures verify strict audit rejects open-ended fuzz claims backed only by deterministic example corpora.
- `semantic_revision_detects_direct_canonical_manifest_drift` verifies a clean commit cannot hide direct canonical edits after RMS apply.
- `strict_trace_coverage_requires_each_named_case_and_workflow_event` verifies strict audit requires every named transition branch and declared workflow event in replay evidence.
- Existing validation, composition, structure, trace, and gate tests cover the deterministic checks reused by audit reports.

The command is provider-independent. It derives production-readiness findings from RMS artifacts and local evidence, lists verification targets, and does not mutate project files.

Source provenance: strict audit records the candidate git revision and rejects dirty, untracked, or incompletely evidenced production claims.

# Law Evidence: project provenance is visible

Promise:

- Fresh RMS workspaces tell agents that production strict audit requires a git source revision.
- `rms diagnose` and `rms gate` surface missing repository or missing commit state before a production claim.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml init_scaffold_generates_valid_system_artifacts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml diagnose_report_includes_config_and_serializes_to_json`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml gate_runs_full_deterministic_checks_outside_git_repository`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml strict_audit_fails_without_source_revision`

Expected result:

- Generated project guidance states that strict audit requires a committed production candidate.
- Diagnose reports a `git source revision` readiness row and guidance when no revision is available.
- Gate reports the no-git full-check fallback and explains that strict audit cannot pass until the project is committed.
- Strict audit continues to fail without a source revision.

Source revision: recorded by git commit before production release.

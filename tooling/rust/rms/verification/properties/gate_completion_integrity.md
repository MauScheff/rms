# Property Evidence: gate completion integrity

Promise:

- `gate-pass-has-no-known-production-blockers`

Scenarios:

- A root with a strict semantic revision blocker makes structural preflight fail.
- A root without a Git source revision runs deterministic checks but cannot pass the gate.
- Dirty and untracked worktree findings are reserved for the final strict audit; semantic and structural findings are not ignored.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml gate_runs_full_deterministic_checks_outside_git_repository`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml gate_structural_preflight_blocks_missing_semantic_revision`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml gate_preflight_ignores_only_final_worktree_provenance`

Expected result:

- All tests pass.
- `rms gate` returns failure when strict semantic or structural blockers exist or no source revision can be resolved.
- A gate pass never hides a known production blocker; clean committed provenance remains the final `rms audit --strict` responsibility.

Source revision: resolved by strict audit from the committed candidate.

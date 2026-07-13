# Property Evidence: initialization establishes provenance

Promise:

- `bootstrap-creates-provenance-worktree`

Scenarios:

- Initializing a fresh standalone directory creates the RMS artifacts, local skills, and a Git worktree.
- Initializing inside an existing parent worktree reuses the parent and does not create a nested repository.
- Generated guidance requires a bootstrap revision, a successful gate, a candidate commit, and a successful strict audit.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml init_scaffold_generates_valid_system_artifacts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml init_reuses_parent_git_worktree_without_nested_repository`

Expected result:

- Both tests pass.
- Standalone roots contain `.git`; nested roots resolve to the existing parent worktree without a child `.git`.
- Generated `AGENTS.md` states the non-bypassable semantic gate and binary completion sequence.

Source revision: resolved by strict audit from the committed candidate.

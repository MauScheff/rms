# Contract Evidence: initialization and authorized provenance

Promise:

- `init-rms-system` initializes a fresh target or explicitly adopts an existing repository without overwriting project-owned content.
- Initialization prepares a Git provenance worktree but does not create a commit or grant Git authority.
- A successful bootstrap reports exactly `bootstrap prepared; provenance baseline pending authorized commit` and orders onboarding as init, authorized bootstrap commit, design, then the recommended scaffold.

Deterministic scenarios:

- Initialize a standalone empty directory and verify canonical artifacts, managed guidance, and a Git worktree are prepared without a commit.
- Initialize beneath an existing parent worktree and verify no nested `.git` directory is created.
- Adopt an existing repository with project-owned documents, synchronize the managed guidance twice, and prove content outside managed markers remains byte-identical.
- Exercise strict-init collisions, invalid existing canonical artifacts, malformed managed sections, and a late managed-skill conflict. Preflight failure writes no earlier artifact.
- Inspect command output and generated onboarding for the exact pending-state and ordering requirements; assert no output claims completion or commit permission.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked init_scaffold_generates_valid_system_artifacts -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked init_reuses_parent_git_worktree_without_nested_repository -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked init_adopt_preserves_existing_documents_and_creates_only_missing_artifacts -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked init_adopt_conflict_preflight_writes_nothing -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked init_reports_bootstrap_pending_authorized_commit -- --nocapture
```

Acceptance oracle:

- Standalone initialization establishes a worktree with no source revision; nested initialization reuses its ancestor worktree and creates no nested repository.
- Successful output contains the exact bootstrap pending state. It contains no executed `git commit`, no assertion that a provenance commit exists, and no production-ready claim.
- Generated onboarding places the authorized bootstrap commit between init and design, with the recommended standalone or recursive scaffold only after design.
- Adoption preserves project-owned bytes outside one valid managed section and remains idempotent.
- Every preflight rejection leaves the fixture tree unchanged and names the conflicting or invalid input explicitly.

Verification status: this file declares the deterministic proof protocol and does not assert an observed pass. Source provenance and executed results are resolved from the authorized candidate commit by `rms audit --root . --strict`.

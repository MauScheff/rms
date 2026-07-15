# Contract Evidence: managed agent guidance and commit authority

Promise:

- `configure-agent-integration` renders full, adopted, and Claude guidance from canonical crate assets and synchronizes only RMS-managed guidance and skills.
- Agent diagnosis distinguishes `exact-current`, `managed-current`, `drifted`, `malformed`, `unmanaged`, and `missing` guidance.
- Guidance treats Git commits as required evidence only when the task and host policy grant commit authority; it never grants that authority itself.

Deterministic scenarios:

- Generate full `AGENTS.md`, adopted managed-section content, and `CLAUDE.md`; compare bytes with their included assets. Assert full guidance has at most 100 lines and 12 KiB and contains exactly the authority, start/route, change gate, hard boundaries, and completion sections.
- Diagnose exact generated guidance, a current managed section surrounded by project content, drifted managed text, duplicated or unbalanced markers, unmanaged project guidance, and an absent file.
- Synchronize an adopted document twice and prove the project-owned prefix/suffix are byte-identical, the managed block is singular, and the second synchronization is idempotent.
- Add unrelated local skills beside the RMS-managed catalog and prove synchronization preserves them while missing or divergent managed skills are refreshed.
- Inspect detected origins and verify unknown runtime activation and host-defined precedence remain explicit after init and sync.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked agent_guidance_assets_are_exact_and_within_budgets -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked agent_guidance_status_distinguishes_all_managed_states -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked agent_sync_preserves_adopted_content_and_unrelated_skills -- --nocapture
```

Acceptance oracle:

- Generated files equal their canonical assets, meet both size limits, and contain the exact commit-authority policy and the two prescribed pending-state strings.
- Every fixture maps to exactly one guidance state; malformed markers are never silently overwritten as valid managed guidance.
- Sync changes only the RMS-managed document section and expected RMS skill paths. Unrelated skill files and project text retain their original bytes.
- Equivalent origins are informational and divergent origins require review with deterministic remediation.
- Neither generated guidance nor diagnosis claims a Git commit occurred, grants commit permission, claims production readiness before strict audit, or promotes a detected source to runtime-active.

Verification status: this file declares the deterministic proof protocol and does not assert an observed pass. Source provenance and executed results are resolved from the authorized candidate commit by `rms audit --root . --strict`.

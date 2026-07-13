# Semantic Revision History Is Sealed

Promise:

- semantic-revision-history-is-sealed

Scenarios:

- Apply two semantic changes to a module-only fixture. The second record automatically supersedes the first, both files remain present, and the module stores the SHA-256 digest of the latest exact record.
- Apply after two independent active semantic records. The new revision supersedes both even when an intervening machine or surface seal is the latest metadata pointer.
- Modify a sealed record after apply. Strict revision audit reports `semantic.revision-record-drift` before canonical semantics are trusted.
- Reference a missing superseded record. Semantic apply rejects the change with `semantic-change.supersedes-missing`.

Command/tool:

- `cargo test -p rms --locked spec_apply_auto_chains_and_seals_module_only_semantic_revisions`
- `cargo test -p rms --locked spec_apply_supersedes_every_active_semantic_revision`
- `cargo test -p rms --locked semantic_revision_detects_direct_canonical_manifest_drift`
- `cargo test -p rms --locked semantic_change_rejects_missing_superseded_revision`

Expected result:

- Applied semantic history remains append-only and tampering or broken history is deterministic audit failure.

Source revision: resolved by the final clean commit and `rms audit --root . --strict`.

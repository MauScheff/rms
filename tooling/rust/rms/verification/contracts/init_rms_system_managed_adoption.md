# Contract Evidence: managed existing-repository adoption

Promise: `rms init --adopt` integrates RMS into an existing repository without erasing project-owned documents, and later agent synchronization remains confined to the RMS-managed guidance section.

Scenarios:

- `init_adopt_preserves_existing_documents_and_creates_only_missing_artifacts` verifies the glossary remains byte-identical, existing agent and ignore content remains the exact file prefix, one managed section is installed, missing RMS artifacts are created, and `rms agent sync` retains the original agent content.
- `managed_section_merge_preserves_outside_content_and_is_idempotent` replaces only a marked RMS section, preserves exact prefix and suffix content, and produces the same result when repeated.
- `init_adopt_accepts_an_already_compatible_rms_workspace` verifies adoption is idempotent over a complete RMS-generated workspace.
- `init_adopt_conflict_preflight_writes_nothing` and `init_adopt_rejects_incompatible_canonical_manifests_before_writing` verify conflicts abort before any missing RMS artifact is created.
- `init_without_adopt_refuses_existing_documents_before_writing` verifies normal initialization remains collision-intolerant.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked init_ -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked managed_section_merge_preserves_outside_content_and_is_idempotent
```

Expected result: every test passes; adoption reports `created`, `adopted`, or `updated`; project content outside managed markers is unchanged; managed content is singular and idempotent; strict initialization and conflict preflight write nothing.

Source provenance: the production claim is bound to the committed candidate revision resolved by `rms audit --root . --strict`.

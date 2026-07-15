# Contract Evidence: safe RMS adoption

Promise: `init-rms-system` may adopt an existing repository without overwriting accepted files or leaving a partial RMS bootstrap after a preflight conflict.

Scenarios:

- `init_adopt_preserves_existing_documents_and_creates_only_missing_artifacts` starts with project-owned `GLOSSARY.md`, `AGENTS.md`, and `.gitignore`, runs adoption, and verifies the glossary remains byte-for-byte identical, the other project-owned content remains intact outside marked RMS sections, and missing canonical artifacts, workbench configuration, local skills, and Git provenance are created.
- `init_adopt_accepts_an_already_compatible_rms_workspace` reruns adoption over a complete compatible RMS workspace and verifies the existing canonical system manifest is unchanged.
- `init_without_adopt_refuses_existing_documents_before_writing` proves strict initialization still refuses a collision and creates no canonical manifests.
- `init_adopt_conflict_preflight_writes_nothing` places a conflict at a late RMS-managed skill path and verifies that no earlier canonical artifact or workbench configuration was written.
- `init_adopt_rejects_incompatible_canonical_manifests_before_writing` proves an unrelated `system.yaml` is reported as a conflict, remains unchanged, and prevents all missing RMS artifacts from being written.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked init_ -- --nocapture
```

Expected result: all initialization and adoption tests pass; project-owned content is retained, missing files are generated, incompatible managed content is rejected before writes, Git initialization remains non-nesting, and every successful path prints a per-artifact `created`, `adopted`, or `updated` status.

Source provenance: the production claim is bound to the committed candidate revision resolved by `rms audit --root . --strict`.

# Contract Evidence: validate-rms-artifacts

Covered by `cargo test --workspace --locked` and repository-level `rms validate --root .`, which exercise manifest parsing, schema validation, reference checks, compatibility checks, implementation binding diagnostics, structure role alias normalization, advisory numeric-safety diagnostics, and cross-module private-role import diagnostics.

Focused regressions:

```sh
cargo test --workspace --locked generated_contract_semantics_are_visible_scaffold_obligations
cargo test --workspace --locked local_workspace_evidence_is_source_unpinned_without_git_revision
```

Expected result: generated contract meaning is reported as `semantic.contract-scaffold-active`, while evidence that claims an uncommitted workspace or missing Git revision is reported as `evidence.source-unpinned`.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

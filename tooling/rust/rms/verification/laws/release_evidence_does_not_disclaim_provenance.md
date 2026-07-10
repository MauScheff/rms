# Release Evidence Does Not Disclaim Provenance

Promise: evidence used by a production claim cannot describe itself as an uncommitted workspace, a current filesystem snapshot, or a repository without a Git revision.

Scenario: inspect each unsupported provenance phrase observed in blind dogfood evidence.

Command:

```sh
cargo test --workspace --locked local_workspace_evidence_is_source_unpinned_without_git_revision
```

Expected result: every phrase produces `evidence.source-unpinned`, and strict audit converts that diagnostic into a release failure.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

# Law Evidence: complete provenance

Promise: `strict-audit-observes-all-production-files`.

Scenarios:

- A wholly untracked nested module is included in the changed-path set.
- Declared role files, contracts, evidence, package files, build scripts, and runnable entrypoints participate in provenance and semantic-drift checks.

Command/tool:

- `cargo test --workspace --locked untracked`
- `rms audit --root . --strict`

Expected result: untracked or dirty production artifacts prevent a clean production claim and identify the exact paths involved.

Source revision: supplied by the enclosing committed RMS candidate and checked by strict audit.

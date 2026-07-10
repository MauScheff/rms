# Property Evidence: canonical semantic revision

Promise: `canonical-semantics-match-cli-applied-revision`.

Input space: RMS module and implementation manifests sealed by spec, machine, or surface apply, followed by unchanged and directly mutated canonical projections.

Oracle:

- unchanged module, implementation, and contract semantics reproduce the recorded SHA-256 digest;
- a direct canonical edit produces `semantic.revision-drift`;
- a missing record or revision fails strict audit.

Command/tool: `cargo test --workspace --locked semantic_revision_detects_direct_canonical_manifest_drift`.

Expected result: the RMS-applied candidate passes revision integrity and the directly edited candidate fails.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

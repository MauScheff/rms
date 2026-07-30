# Property Evidence: canonical semantic revision

Promise: `canonical-semantics-match-authorized-revision`.

Input space: RMS module and implementation manifests sealed by spec, machine,
or surface apply; an ordinary module falsely labeled with repository maintainer
authority; the explicitly declared RMS self-development module sealed by the
repository maintainer workflow; and directly mutated canonical projections.

Oracle:

- unchanged module, implementation, and contract semantics reproduce the recorded SHA-256 digest;
- repository maintainer authority is rejected without the explicit RMS
  module identity, self-development declaration, evidence, and public ownership
  prerequisites;
- the declared RMS self-development module accepts the repository maintainer
  seal while retaining the same immutable record and projection checks;
- a direct canonical edit produces `semantic.revision-drift`;
- a missing record or revision fails strict audit.

Command/tool: `cargo test --workspace --locked semantic_revision_detects_direct_canonical_manifest_drift`.

Expected result: RMS-applied and valid repository-maintainer-sealed candidates
pass revision integrity; false self-application authority and directly edited
candidates fail.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

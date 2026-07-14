# Law Evidence: artifact transformation contracts

Promise: `artifact-transformations-preserve-declared-contracts`.

Scenarios: a required artifact resolves to one provider with matching semantic name, version, and contract identity; missing, incompatible, and ambiguous providers are rejected.

Command/tool: `cargo test -p rms` (including `compose_matches_required_and_provided_artifact_contracts` and `spec_apply_writes_universal_semantics_to_canonical_artifacts`).

Observed result: the semantic apply fixture wrote artifact and transformation declarations to canonical manifests, and composition accepted the compatible edge. The 262-test RMS suite passed.

Source provenance: the clean committed candidate revision resolved by strict audit.

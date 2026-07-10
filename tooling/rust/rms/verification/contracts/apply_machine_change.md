# Contract Evidence: apply-machine-change

Promise: `apply-machine-change` validates the complete post-change candidate before updating `implementation.yaml`, records the exact change, and reseals canonical semantics. It never manufactures active replay evidence from the transition declarations being applied.

Scenario: valid YAML adds or revises semantic variants, named transition cases, effect protocols, and roles. A malformed optional field may be repaired when the final candidate is valid. Invalid references, unclassified inputs, stale removals, and incomplete aggregate protocols fail before mutation.

Command: `cargo test --workspace --locked machine_apply -- --nocapture`

Expected result: valid changes update one canonical machine model used by validation, dry-run reporting, and writes. Optional fields are omitted rather than serialized as invalid nulls. No `verification/traces/machine_change.yaml` is generated; implementation tests and declared replay roles must supply independent trace evidence.

Source provenance: the candidate commit and this command are recorded by strict audit before a production claim.

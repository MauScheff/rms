# Contract Evidence: apply-machine-change

Promise: `apply-machine-change` validates the complete post-change candidate before updating `implementation.yaml`, derives binding container and envelope types from canonical semantics, records the exact change, and reseals the revision. It never manufactures active replay evidence from transition declarations.

Scenario: valid YAML adds or revises semantic variants, named transition cases, exact transition-record ownership, effect protocols, and roles. Effectful stateful candidates require exact driver and transition-record functions. Invalid references, unclassified inputs, stale removals, and incomplete protocols fail before mutation.

Command: `cargo test --workspace --locked machine_apply -- --nocapture`

Expected result: valid changes update one canonical machine model used by validation, dry-run reporting, and writes. Binding type mappings remain distinct from semantic variants, exact live record functions are written, and no `verification/traces/machine_change.yaml` is generated; implementation tests and declared replay roles supply independent trace evidence.

Source provenance: the candidate commit and this command are recorded by strict audit before a production claim.

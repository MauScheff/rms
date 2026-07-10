# Contract Evidence: apply-machine-change

Promise: `apply-machine-change` validates the complete `rms/machine-change/v0.1` model before updating `implementation.yaml`, binding roles, and trace evidence. Semantic variants remain distinct from binding type names; stateful machines declare `state-and-input`; effects declare atomic protocols and typed results.

Scenario: valid YAML adds semantic variants, observed events, type names, transitions, and effect protocols. Invalid references, collapsed type/case declarations, unclassified inputs, and incomplete aggregate protocols fail before mutation.

Command: `cargo test --workspace --locked machine_apply -- --nocapture`

Expected result: valid changes update one canonical machine model used by validation, dry-run reporting, and writes. Dry runs do not mutate files; unknown states and invalid protocol references are rejected.

Source provenance: the candidate commit and this command are recorded by strict audit before a production claim.

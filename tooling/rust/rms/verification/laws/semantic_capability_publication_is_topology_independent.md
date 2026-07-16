# Law Evidence: capability publication is topology-independent

Promise: `semantic-capability-publication-is-topology-independent` means an existing standalone module can publish its first provided capability through `rms spec apply` and a matching public behavior binding without creating a composite or child directories.

Scenario: a Swift-bound Client Account Access domain module adds a capability-kind contract, updates `provides.capabilities`, and records its public behavior path in one semantic change.

Command/tool:

- `cargo test -p rms spec_apply_publishes_first_capability_on_standalone_module`
- `cargo test -p rms removed_add_capability_command_is_unavailable`

Expected result: publication passes on the standalone module, while the removed ambiguous `add-capability` command is unavailable and `add-capability-tree` remains the explicit recursive scaffold.

Source revision: strict audit binds this evidence to the committed candidate.

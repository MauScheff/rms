# Scenario Evidence: semantic-only module to implementation binding

Promise:

- `add-rms-binding` realizes an existing semantic shape without replacing its module semantics.

Scenario:

- Scaffold a `domain-engine` without a binding and retain the exact module manifest.
- Attach the JS binding through `run_add_binding`, inspect its domain-named machine, and compare the original and final module manifests.
- Exercise duplicate and destination-conflict rejection paths.

Command/tool:

- `cargo test -p rms add_binding -- --nocapture`
- `cargo test --workspace --locked`

Expected result:

- The successful binding contains `implementation.yaml`, representation, transition, public facade, tests, and build/verify scripts from the JS binding adapter.
- The canonical module manifest remains byte-identical.
- Duplicate attachment is explicit, and a conflicting path installs no partial files.

Source revision: recorded by git commit or strict audit provenance before production use.

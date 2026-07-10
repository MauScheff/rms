# Contract Evidence: check-machine-structure

Promise: `check-machine-structure` reports whether semantic cases, binding types, the closed machine input, state-and-input transition signature, effect protocols, transition output, replay support, and trace roles agree.

Scenario: generated Rust, Swift, and JavaScript bindings realize the same canonical model idiomatically. Malformed fixtures collapse cases into type names, omit state/input parameters, bypass effect results, hide orchestration in executors, or leave trace branches uncovered.

Command: `cargo test --workspace --locked`

Expected result: structure gaps produce focused deterministic diagnostics. Strict checks reject collapsed semantic variants, missing machine inputs, stateful transitions without state, incomplete effect protocols, result paths that bypass transitions, hidden executor orchestration, role drift, and replay gaps.

Source provenance: the candidate commit and this command are recorded by strict audit before a production claim.

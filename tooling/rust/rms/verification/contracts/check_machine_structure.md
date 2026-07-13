# Contract Evidence: check-machine-structure

Promise: `check-machine-structure` reports whether semantic cases, binding types and envelopes, the closed machine input, exact state-and-input transition-record path, effect protocols, live record retention, total transition arithmetic, replay support, and trace roles agree.

Scenario: generated Rust, Swift, and JavaScript bindings realize the same canonical model idiomatically. Malformed fixtures omit declared envelopes, return only transition outputs from live drivers, use unchecked numeric progress, collapse cases into type names, bypass effect results, hide orchestration in executors, or leave trace branches uncovered.

Command: `cargo test --workspace --locked`

Expected result: structure gaps produce focused deterministic diagnostics. Strict checks reject missing envelope representations, incomplete record paths, output-only live history, unchecked transition arithmetic, collapsed semantic variants, result paths that bypass transitions, hidden executor orchestration, role drift, and replay gaps.

Source provenance: the candidate commit and this command are recorded by strict audit before a production claim.

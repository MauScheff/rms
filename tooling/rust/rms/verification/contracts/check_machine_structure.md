# Contract Evidence: check-machine-structure

Promise: `check-machine-structure` reports whether declared machine roles, variants, transition output, envelopes, replay support, and trace roles are present.

Scenario: generated Rust, Swift, and JavaScript bindings pass structure checks; malformed fixtures produce focused diagnostics.

Command: `cargo test --manifest-path tooling/rust/rms/Cargo.toml structure_report js_boundary_machine_flags -- --nocapture`

Expected result: structure gaps are deterministic diagnostics, including missing transition output, string-only JS state, role drift, hidden effects, and reply-only boundary machines.

Source revision: git:dfe027ab8502 plus current semantic-gate change under review.

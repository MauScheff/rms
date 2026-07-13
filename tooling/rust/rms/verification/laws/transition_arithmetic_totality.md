# Law Evidence: transition arithmetic is total

Promise:

- `transition-arithmetic-is-total`: arithmetic over represented transition inputs is checked or bounded, so an extreme valid input yields a declared output or rejection rather than panic, trap, wrap, or silent loss.

Scenario:

- Rust and JavaScript fixtures increment a represented batch index at its numeric maximum inside the exact transition path.
- Matching fixtures replace unchecked addition with checked arithmetic and an explicit rejection branch.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_transition_arithmetic_distinguishes_unchecked_and_checked_indices`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml js_binding_rejects_unchecked_transition_index_arithmetic`

Expected result:

- Unchecked fixtures report `structure.transition-unchecked-arithmetic`.
- Checked or bounded fixtures do not report that diagnostic and retain an explicit rejection result for overflow.

Source revision: resolved from the candidate Git commit by strict audit.

# Property Evidence: transition-arithmetic-totality

Promise:

- Property `transition-arithmetic-totality` proves `transition-arithmetic-is-total`.

Input space:

```yaml
fixtures:
- unchecked index increment at the numeric maximum
- checked increment with explicit rejection
```

Oracle:

- unchecked arithmetic in a transition is rejected
- checked or bounded arithmetic remains explicit and total

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml rust_transition_arithmetic_distinguishes_unchecked_and_checked_indices`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml js_binding_rejects_unchecked_transition_index_arithmetic`

Expected result:

- Unchecked maximum-index increments report `structure.transition-unchecked-arithmetic`.
- Checked or bounded increments pass and preserve an explicit rejection path.
- Any future failing fixture is recorded under `verification/fuzz/counterexamples/transition-arithmetic` with `spec: rms/property-counterexample/v0.1`.

Source revision: recorded by git commit or strict audit provenance before production use.

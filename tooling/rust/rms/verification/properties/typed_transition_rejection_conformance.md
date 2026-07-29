# Property Evidence: typed-transition-rejection-conformance

Promise:

- Property `typed-transition-rejection-conformance` proves `expected-rejections-remain-typed`.

Input space:

```yaml
fixtures: inspectable transition output types with and without explicit rejection fields
```

Oracle:

- declared rejection variants require an explicit rejection output field
- branch-only rejection encoding fails

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml rejection_channel`

Observed result:

- Rust, Swift, JavaScript, and Python fixtures without rejection-bearing transition output are all rejected with `structure.transition-rejection-channel-missing`.

Source revision: recorded by git commit or strict audit provenance before production use.

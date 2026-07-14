# Property Evidence: boundary-effect-protocol-conformance

Promise:

- Property `boundary-effect-protocol-conformance` proves `effectful-boundaries-use-explicit-protocols`.

Input space:

```yaml
fixtures: boundary adapters with declared and hidden filesystem process clock and network IO
```

Oracle:

- inspectable IO requires declared effects results protocol and executor ownership
- pure no-effect boundaries remain valid

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml inspectable_boundary_io_requires_a_protocol_and_exact_surface_callable`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml boundary_adapter_scaffold`

Observed result:

- Hidden local file IO without a protocol fails; generated no-effect parser boundaries continue to pass their scaffold verification.

Source revision: recorded by git commit or strict audit provenance before production use.

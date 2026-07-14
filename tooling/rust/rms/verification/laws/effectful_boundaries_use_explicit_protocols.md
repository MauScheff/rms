# Law Evidence: effectful boundaries use explicit protocols

Promise:

- Inspectable boundary IO is represented by declared effects, typed results, atomic protocols, and dedicated executor ownership.

Scenario:

- A Rust boundary adapter reads a local file but declares no machine effect protocol or executor.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml inspectable_boundary_io_requires_a_protocol_and_exact_surface_callable`

Expected result:

- RMS reports `structure.boundary-io-without-effect-protocol`.

Source revision: recorded by git commit or strict audit provenance before production use.

# Law Evidence: runnable surfaces execute declared machines

Promise:

- An inspectable runnable surface delegates to an exact callable that reaches the declared machine path.

Scenario:

- A runnable Rust surface names only `src/lib.rs` as its delegate while a machine is declared.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml inspectable_boundary_io_requires_a_protocol_and_exact_surface_callable`

Expected result:

- RMS reports `structure.runnable-surface-machine-entrypoint-missing` for file-only delegation.

Source revision: recorded by git commit or strict audit provenance before production use.

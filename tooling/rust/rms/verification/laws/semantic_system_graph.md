# Evidence: semantic system graph is cross-layer and source-backed

Promise:

- semantic-system-graph-is-cross-layer-and-source-backed

Scenario:

- Project a pure implemented module containing module-level promises and an implementation-level machine.
- Inspect the resulting graph for module, implementation, machine, state, input, output, semantic-function, and source-provenance nodes and edges.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml semantic_graph -- --nocapture`
- `cargo run --manifest-path tooling/rust/rms/Cargo.toml -- validate --root . --json`

Expected result:

- The graph unit fixtures expose inner machine semantics alongside module contracts and ownership.
- Every projected node and edge carries a stable identifier and source reference derived from canonical RMS artifacts.
- Repository validation reports no semantic graph closure diagnostic for a resolved chain.

Source revision: resolved from the candidate Git commit by `rms audit --root . --strict`.

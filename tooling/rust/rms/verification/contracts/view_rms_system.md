# Contract Evidence: `view-rms-system`

Promise:

- `rms view` serves a local semantic explorer derived from discovered RMS modules.
- The snapshot exposes one cross-layer semantic graph spanning module topology, contracts, implementation machines, semantic functions, exact behavior bindings, traces, evidence, source references, diagnostics, and shape-aware obligations.
- The application presents system, behavior, machine, proof, gap, and debug views without inventing a second semantic model.
- Stable deep links, search, browser history, live refresh, and source-backed inspectors preserve exact graph identities and status vocabulary.
- The server binds to loopback and rejects unsupported routes and mutating methods.

Scenario:

- Build the viewer projection from this repository and verify that inner machine semantics, cross-layer bindings, obligation status, and source provenance are present without inferred name-based links.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml viewer --no-fail-fast
cargo test --manifest-path tooling/rust/rms/Cargo.toml semantic_graph --no-fail-fast
cargo run --manifest-path tooling/rust/rms/Cargo.toml -- validate --root . --json
node /tmp/rms-viewer-playwright.mjs
```

Observed result:

- The repository graph includes module and implementation nodes, domain-named machines, semantic cases, exact public/dependency behavior bindings, proof edges, and stable source references.
- Pure and composite module stages that do not own boundary, lifecycle, or effect semantics are `not-applicable`, not false gaps.
- Missing, stale, or unresolved contract-to-machine and consumer-to-provider links surface as deterministic validation diagnostics.
- Playwright exercised all six views at 1440x900, 1024x768, and 390x844; deep-link refresh and browser history preserved selection, compact navigation and inspectors remained usable, no viewport overflow occurred, and no browser errors were emitted.
- The projection and application remain read-only; navigation never becomes semantic authority.

Source revision: resolved from the committed candidate by `rms audit --root . --strict`.

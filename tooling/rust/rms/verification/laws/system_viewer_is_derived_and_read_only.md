# Law Evidence: `system-viewer-is-derived-and-read-only`

Promise:

- Every visible system relationship is derived from canonical composition, module requirements, or matched capability requirements.
- Existing deterministic module-atlas projections supply module nodes, traces, source references, and gaps.
- The viewer exposes no route capable of applying semantics or writing project files.

Scenario:

- Generate a complete system snapshot, compare its declared relationship count with canonical manifests, and exhaust the viewer method/route cross-product.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml viewer::tests::system_view_projects_discovered_modules_with_stable_relationships
cargo test --manifest-path tooling/rust/rms/Cargo.toml viewer_request::property_system_viewer_routes_are_read_only
```

Observed result:

- The system projection test discovers canonical modules and emits only stable, source-backed relationship identifiers.
- All 42 exhaustive method/route cases pass: only registered `GET` and `HEAD` routes are accepted; all mutation methods and unknown paths are rejected.
- The request parser has no filesystem, semantic-apply, or process-execution path.

Source revision: resolved from the committed candidate by `rms audit --root . --strict`.

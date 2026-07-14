# Contract Evidence: `view-rms-system`

Promise:

- `rms view` serves a local semantic explorer derived from discovered RMS modules.
- The snapshot exposes module topology, semantic objects, traces, evidence, source references, diagnostics, and explicit gaps.
- The server binds to loopback and rejects unsupported routes and mutating methods.

Scenario:

- Build the viewer from this repository, serve its root in watch mode, inspect the five journeys in a browser, and probe the closed HTTP route table.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml viewer --no-fail-fast
target/debug/rms view --root . --watch --no-open --port 0
curl -i http://127.0.0.1:<port>/api/snapshot
curl -I http://127.0.0.1:<port>/api/health
curl -i -X POST http://127.0.0.1:<port>/api/snapshot
curl -i http://127.0.0.1:<port>/unknown
```

The in-app browser exercised Understand, Trace, Change, Debug, and Verify at desktop and 390-pixel mobile widths.

Observed result:

- The repository projected 9 canonical modules, 3 declared relationships, 505 semantic objects, 58 guided traces, and 42 explicit gaps.
- `GET /api/snapshot` and `HEAD /api/health` returned `200`.
- `POST /api/snapshot` returned `405`; an unknown route returned `404`.
- Browser inspection found no console errors or horizontal overflow.

Source revision: resolved from the committed candidate by `rms audit --root . --strict`.

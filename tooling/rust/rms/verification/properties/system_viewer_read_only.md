# Property Evidence: `system-viewer-read-only-route-property`

Promise:

- The finite viewer route table accepts only read-only `GET` and `HEAD` requests.
- Mutation methods and unknown targets always produce an explicit rejection.

Scenario:

- Generate the Cartesian product of 7 HTTP methods and 6 request targets: five registered paths plus one unknown path.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml viewer_request::property_system_viewer_routes_are_read_only
```

Observed result:

- 42/42 cases passed.
- Registered `GET` and `HEAD` requests map to a closed response kind.
- `POST`, `PUT`, `PATCH`, `DELETE`, and `OPTIONS` always return method-not-allowed.
- Unknown targets always return not-found; a query string does not expand the registered route table.

Counterexample policy: the failing method and target are printed directly by the Rust assertion.

Source revision: resolved from the committed candidate by `rms audit --root . --strict`.

# Evidence: contract proves recommend-next-rms-work

Promise:

- recommend-next-rms-work

Scenario:

- Exercise valid, repairable, operationally failed, ambiguous-owner, blocked, and ready routes.
- Inspect every route's artifacts, owner projection, receipt actions, next step, and exit class.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml provider_pipeline_repairs_caches_refreshes_and_deduplicates --no-fail-fast`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml non_ready_routes_never_select_an_owner --no-fail-fast`

Expected result:

- Operational failure records are complete and non-authorizing.
- Non-ready routes expose no selected owner or owner-scoped implementation step.
- Ready routes retain deterministic owner selection and receipt-gated actions.

Source revision: recorded by git commit or strict audit provenance before production use.

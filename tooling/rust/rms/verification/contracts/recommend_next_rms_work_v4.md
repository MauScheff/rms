# Evidence: contract proves recommend-next-rms-work

Promise:

- recommend-next-rms-work

Scenario:

- Exercise valid, repairable, quota-exhausted, read-only-state, operationally failed, ambiguous-owner, blocked, and ready routes.
- Exercise default provider selection and a one-run Codex CLI profile.
- Inspect every route's artifacts, owner projection, receipt actions, next step, and exit class.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml provider_pipeline_repairs_caches_refreshes_and_deduplicates --no-fail-fast`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml non_ready_routes_never_select_an_owner --no-fail-fast`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml exact_observation_source_repair_routes_despite_unrelated_validation_debt --no-fail-fast`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml provider_intent_normalization_keeps_declared_role_completion_in_implementation_lane --no-fail-fast`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml exact_declared_role_completion_routes_with_task_addressed_implementation_debt --no-fail-fast`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml exact_proof_support_role_addition_routes_despite_unrelated_validation_debt --no-fail-fast`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml owner_scoped_adapter_semantics_route_despite_unrelated_module_debt --no-fail-fast`

Expected result:

- Operational failure records are complete and non-authorizing.
- Quota exhaustion and read-only provider state are classified truthfully and preserve caller-authored recovery.
- A one-run profile is forwarded, audited, and cache-separated without changing ownership policy.
- Non-ready routes expose no selected owner or owner-scoped implementation step.
- Ready routes retain deterministic owner selection and receipt-gated actions.
- An exact named observation-source repair reaches its existing owner while unrelated validation debt remains visible. Intent, ownership, schema, and semantic-revision failures still block it, and later candidate gates remain unchanged.
- An exact existing-module request to complete declared implementation and proof roles while preserving semantics stays in the implementation-candidate lane. Unrelated and task-addressed implementation debt remains visible without erasing the owner, while hard route blockers and final gates remain unchanged.
- An exact existing-module semantic request to add only proof-support roles while preserving public contracts and production behavior reaches its selected owner despite unrelated validation debt. Owner-local errors still block the route, and later candidate gates remain unchanged.
- A complete existing-module semantic request for native adapter lifecycle reaches its selected owner despite errors in an unrelated module. Owner-local, intent, and root-canonical errors still block the route, and later candidate gates remain unchanged.

Source revision: recorded by git commit or strict audit provenance before production use.

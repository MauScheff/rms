# Law Evidence: atomic effect protocols

Promise: `effect-executors-do-not-own-orchestration`.

Scenarios:

- Every effect names its possible results and executor role.
- A one-request-one-result executor containing an orchestration loop is rejected.
- Every declared result is classified as input and consumed by the canonical transition.
- Aggregate protocols require a justification and supporting evidence.

Command/tool:

- `cargo test --workspace --locked effect_protocol`
- `rms machine check tooling/rust/rms/implementation.yaml --strict`

Expected result: missing protocols, unhandled results, executor-owned transitions, and unjustified aggregates fail deterministic checks.

Source revision: supplied by the enclosing committed RMS candidate and checked by strict audit.

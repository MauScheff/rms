# Law Evidence: trace records conform to canonical transitions

Promise:

- Every checked trace record names a canonical transition case and realizes its declared state change, events, commands, effects, reply, and rejection.

Scenario:

- A matching generated trace passes, while a record whose `Accept` case emits `Rejected` fails with `trace.canonical-transition-mismatch`.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml trace_bundle_rejects_outputs_that_do_not_match_the_canonical_case`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml scaffold_generates`

Expected result:

- The mismatch fixture is rejected and generated binding trace bundles pass canonical conformance.

Source revision: recorded by git commit or strict audit provenance before production use.

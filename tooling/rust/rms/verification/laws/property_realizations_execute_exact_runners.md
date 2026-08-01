# Law Evidence: property realizations execute exact runners

Promise:

- Law `property-realizations-execute-exact-runners` requires one execution result per declared realization, even when commands are shared.

Scenario:

- Two smoke properties share one executable command but declare different runner identities.
- Generated-property fixtures include valid, missing, unresolved, and fixed-corpus generators and runners.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml property_run_executes_each_shared_command_realization`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml property_realizations_require_generators_runners_operations_and_oracles`

Expected result:

- The shared command executes twice with distinct `RMS_PROPERTY_ID` and `RMS_PROPERTY_RUNNER` values.
- Generated JavaScript dispatch through `process.env.RMS_PROPERTY_RUNNER` is recognized as exact runner selection.
- Generated Python dispatch through `tests/rms_proof_runner.py RMS_PROPERTY_RUNNER` is recognized as exact runner selection while an arbitrary command containing the bare environment name is not.
- Missing generator, runner, operation, and oracle ownership produce distinct diagnostics.

Observed result: both focused regressions passed. Source revision is resolved from the committed candidate by strict audit.

# Law Evidence: effect results are canonical machine inputs

Promise:

- `effect-results-are-canonical-machine-inputs`
- Effect outcomes that drive transitions are declared as effect results and are consumed by transition declarations.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml structure_report_flags_effect_result_representation_drift`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml structure_report_flags_outcome_transition_inputs_not_declared_as_effect_results`

Expected result:

- RMS reports effect-result drift when represented effect outcomes or outcome-like transition inputs are not reflected in `architecture.machine.effect_results`.

Source revision:

- Recorded by git commit before a production claim.

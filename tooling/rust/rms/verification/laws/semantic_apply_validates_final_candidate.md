# Law Evidence: semantic apply validates the final candidate

Promise:

- Law `semantic-apply-validates-final-candidate` requires apply commands and JSON Schema to share one language-neutral semantic-ID grammar and to compute and validate one repaired canonical model before dry-run reporting or writes.

Scenario:

- Start from a machine containing invalid legacy or stale structure, then submit a change whose final candidate is valid.
- Submit a candidate that remains invalid after all set, remove, and add operations.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml machine_apply_validates_the_repaired_candidate`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml machine_apply_rejects_unreachable_final_states_before_write`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml spec_apply_writes_universal_semantics_to_canonical_artifacts`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml spec_apply_repairs_schema_invalid_current_artifact_state`

Expected result:

- A valid repaired final model writes successfully even when the starting manifest is invalid.
- Hyphenated artifact and transformation IDs accepted by semantic apply are accepted by the module schema.
- Invalid final models fail before any artifact write.

Observed result: all focused regressions passed, including dry-run non-mutation and schema validation of the repaired final candidate. Source revision is resolved from the committed candidate by strict audit.

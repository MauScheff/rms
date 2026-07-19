# Property evidence: check proof projection truthfulness

Input space crosses declared property strategies, trace producers, generic native verification, explicit strict-audit observations, and project/progressive/module coverage modes.

Oracle:

- declarations never enter `observed` without a matching execution record
- generic verification never implies a property strategy or trace producer execution
- exact observed strategies and profiles suppress only the matching declaration

Execution:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml check_proof_projection_never_promotes_declarations`

The deterministic regression corpus passes with declarations retained separately from observed validators.

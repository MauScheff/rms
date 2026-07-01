# Contract Evidence: apply-machine-change

Promise: `apply-machine-change` validates `rms/machine-change/v0.1` before updating `implementation.yaml`, role scaffolds, and trace evidence.

Scenario: valid YAML adds states, commands, effects, transitions, and an effect executor; invalid references fail before mutation.

Command: `cargo test --manifest-path tooling/rust/rms/Cargo.toml machine_apply -- --nocapture`

Expected result: valid machine changes update canonical machine declarations; dry runs do not write; unknown states are rejected.

Source revision: git:dfe027ab8502 plus current semantic-gate change under review.

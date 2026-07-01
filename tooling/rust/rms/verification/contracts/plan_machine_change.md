# Contract Evidence: plan-machine-change

Promise: `plan-machine-change` renders an advisory `rms/machine-change/v0.1` prompt without making provider output semantic authority.

Scenario: `rms machine plan <implementation.yaml> --task "<intent>"` prints the current machine, gaps, expected output schema, and deterministic checks.

Command: `cargo test --manifest-path tooling/rust/rms/Cargo.toml machine_apply workbench_prompt -- --nocapture`

Expected result: machine planning remains advisory; only `rms machine apply` mutates canonical artifacts.

Source revision: git:dfe027ab8502 plus current semantic-gate change under review.

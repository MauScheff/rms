# Contract Evidence: plan-semantic-change

Promise: `plan-semantic-change` renders an advisory `rms/semantic-change/v0.1` prompt for laws, contracts, machine structure, runnable surfaces, effects, and evidence obligations.

Scenario: `rms spec plan <module.yaml|implementation.yaml> --task "<intent>"` asks providers to output a semantic-change object, not direct architecture edits.

Command: `cargo test --manifest-path tooling/rust/rms/Cargo.toml workbench_prompt implement_prompt refactor_prompt -- --nocapture`

Expected result: prompts say provider output is advisory until `rms spec apply` updates canonical artifacts.

Source revision: git:dfe027ab8502 plus current semantic-gate change under review.

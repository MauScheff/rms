# Contract Evidence: plan-semantic-change

Promise: `plan-semantic-change` renders a self-contained advisory `rms/semantic-change/v0.1` prompt for laws, contracts, machine structure, runnable surfaces, effects, and evidence obligations.

Scenario: render semantic and focused machine plans for fresh scaffolded implementations without consulting another project or RMS source.

Command: `cargo test --manifest-path tooling/rust/rms/Cargo.toml plan_prompt_is_self_contained -- --nocapture`

Expected result: the semantic prompt enumerates every invariant authority, shows exact `set`, `add`, and `remove` forms, explains structured transition removal and scaffold replacement, and both prompts prohibit external template lookup. Provider output remains advisory until the corresponding RMS apply command succeeds.

Source revision: resolved by the candidate commit and `rms audit --root . --strict`.

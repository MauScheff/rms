# Property Evidence: semantic-plan-prompt-is-actionable-without-external-examples

Promise: `semantic-plan-schema-is-self-contained`.

Scenario: render `rms spec plan` and `rms machine plan` prompts for fresh scaffolded modules using only current-project artifacts.

Command: `cargo test --manifest-path tooling/rust/rms/Cargo.toml plan_prompt_is_self_contained -- --nocapture`

Expected result:

- The semantic prompt lists all six invariant authorities and explains `enforced_by`.
- Every machine variant category exposes `set`, `add`, and `remove`.
- Transition removals are documented as structured `from`/`on` objects with optional `to` and `case`.
- Complete scaffold replacement prefers `set` and both prompts prohibit external template lookup.

Source revision: resolved by the candidate commit and `rms audit --root . --strict`.

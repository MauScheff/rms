# Contract Evidence: plan-semantic-change

Promise: `plan-semantic-change` renders a self-contained advisory `rms/semantic-change/v0.1` prompt for laws, ownership-aware provided and required contracts, semantic-function authority bindings, machine structure, runnable surfaces, effects, and evidence obligations.

Scenario: render semantic and focused machine plans for fresh scaffolded implementations, including exact semantic-function add/set/remove operations, without consulting another project or RMS source.

Command: `cargo test --manifest-path tooling/rust/rms/Cargo.toml plan_prompt_is_self_contained -- --nocapture`

Expected result: the semantic prompt enumerates every invariant authority; shows exact `set`, `add`, and `remove` forms for semantic functions and other supported categories; distinguishes `direction: provided|required`, explains that required contracts remain consumer expectations, names allowed function kinds, purity, discharged promises, assumptions, exact symbols, and evidence; renders incremental `surfaces.set` as null; explains structured transition and surface removal plus scaffold replacement; renders implementation-only sections as null for semantic-only modules; and prohibits external template lookup. Provider output remains advisory until the corresponding RMS apply command succeeds.

Source revision: resolved by the candidate commit and `rms audit --root . --strict`.

# Contract Evidence: plan-machine-change

Promise: `plan-machine-change` renders an advisory `rms/machine-change/v0.1` prompt that separates semantic cases from binding types, classifies every machine input, and proposes effect protocols without making provider output semantic authority.

Scenario: `rms machine plan <implementation.yaml> --task "<intent>"` prints the current machine, lifecycle and effect-outcome questions, expected canonical schema, and deterministic checks.

Command: `cargo test --workspace --locked`

Expected result: plans name actual alternatives, use `transition(state, input)` for stateful behavior, keep sequencing out of effect executors, distinguish corpus tests from fuzzing, and remain advisory until `rms machine apply` succeeds.

Source provenance: the candidate commit and this command are recorded by strict audit before a production claim.

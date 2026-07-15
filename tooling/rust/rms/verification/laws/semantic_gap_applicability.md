# Evidence: semantic gaps respect module shape

Promise:

- semantic-gap-applicability-matches-module-shape

Scenario:

- Project a pure stateless domain machine with no effects and no runnable boundary.
- Compare an inapplicable obligation with an applicable but unsatisfied obligation.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml semantic_graph::tests::applicability_distinguishes_absence_from_a_gap -- --exact`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml semantic_graph::tests::pure_machine_projects_inner_semantics_without_effect_gap -- --exact`

Expected result:

- Non-applicable boundary, lifecycle, and effect stages are labeled `not-applicable`, never reported as missing.
- An applicable unsatisfied obligation is labeled `required-gap` and remains visible to validation and strict audit.

Source revision: resolved from the candidate Git commit by `rms audit --root . --strict`.

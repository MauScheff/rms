# Property Evidence: transition case replay

Promise: `transition-cases-are-declared-and-replayed`.

Input space: state/input pairs with one or several semantic destinations and output branches, including effect-result continuation and stop cases.

Oracle:

- every canonical transition has a stable `case`;
- replay state, input, destination, and source branch match that case;
- every declared workflow event appears in replay evidence.

Command/tool: `cargo test --workspace --locked canonical_machine_requires_named_transition_cases` and `cargo test --workspace --locked strict_trace_coverage_requires_each_named_case_and_workflow_event`.

Expected result: unnamed, collapsed, or untraced transition branches fail deterministic checks.

Source provenance: the clean committed candidate revision resolved by strict audit.

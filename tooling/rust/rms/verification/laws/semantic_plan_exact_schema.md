# Law Evidence: self-contained exact planning schema

Promise:

- `semantic-plan-schema-is-self-contained`: rendered plan prompts carry enough type, cardinality, operation, and evidence-linkage information to author changes without external examples.

Scenario:

- Render semantic and machine plans for a binding-backed module and render a semantic plan for a semantic-only module.

Command/tool:

- `cargo test -p rms plan_prompt_is_self_contained -- --nocapture`
- `cargo test -p rms semantic_only_plan_keeps_implementation_sections_null -- --nocapture`

Expected result:

- Prompts enumerate invariant authorities, list/scalar cardinalities, evidence matching, effect protocol atomicity, and structured removals.
- Semantic-only plans render machine, roles, and surfaces as null and name `rms add-binding` as the canonical next step.

Source revision: recorded by git commit or strict audit provenance before production use.

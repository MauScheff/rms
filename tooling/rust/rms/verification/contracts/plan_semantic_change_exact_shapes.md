# Contract Evidence: exact semantic change planning shapes

Promise:

- `plan-semantic-change` renders a self-contained language-neutral change schema with exact value cardinalities and implementation-aware sections.

Scenario:

- Render a plan for a binding-backed module and one for a semantic-only module.

Command/tool:

- `cargo test -p rms plan_prompt_is_self_contained -- --nocapture`
- `cargo test -p rms semantic_only_plan_keeps_implementation_sections_null -- --nocapture`

Expected result:

- Contract fields are identified as scalar strings or non-empty string lists; property oracles and evidence kinds are explicit; each changed law and contract requires matching evidence; effect protocol atomicity is explicit.
- A target without `implementation.yaml` cannot accidentally request machine, role, or surface work from copied empty sections.

Source revision: recorded by git commit or strict audit provenance before production use.

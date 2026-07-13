# Contract Evidence: design binding realization

Promise:

- `design-rms-system` makes the difference between an implemented scaffold and an intentionally semantic-only scaffold explicit.

Scenario:

- Render design guidance for a runnable browser game that needs both invariant-bearing decisions and a boundary.

Command/tool:

- `cargo test -p rms design_prompt_recommends_generic_domain_engine_and_boundary_adapter -- --nocapture`

Expected result:

- The deterministic recommendation includes a recursive tree, explicit domain and boundary binding flags, and `rms add-binding` as the recovery path for intentional deferral.
- It does not prescribe one implementation language as canonical semantics.

Source revision: recorded by git commit or strict audit provenance before production use.

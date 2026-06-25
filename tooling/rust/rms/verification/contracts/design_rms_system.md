# Contract Evidence: design-rms-system

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic design prompt coverage.

Executable coverage:

- `design_prompt_recommends_generic_domain_engine_and_boundary_adapter` verifies `rms.design@v1` renders before a target module exists and recommends a semantic composite tree with a `domain-engine` child plus a `boundary-adapter` child, without hard-coded game-specific module names. It also verifies the prompt requires semantic structure and edge-case decisions before code.
- `design_prompt_allows_boundary_adapter_parser_decisions` verifies adapter-owned parsing, malformed-input rejection, and delegation decisions do not trigger mixed domain/boundary warnings.
- `design_prompt_warns_when_boundary_adapter_owns_domain_rules` verifies boundary modules that also own rule decisions still receive domain-engine split guidance.
- `boundary_adapter_shape_scaffold_gets_boundary_semantics` verifies `rms add-module --shape boundary-adapter` records adapter kind and Boundary-profile obligations by default.
- `validate_warns_when_boundary_adapter_shape_lacks_boundary_semantics` verifies validation warns when scaffold shape and manifest semantics drift apart.
- `workbench_run_record_writes_prompt_request_and_checks` covers the shared run-record path used by advisory design prompts.

Provider execution uses the same rendered prompt and stores provider output under the generated run record.

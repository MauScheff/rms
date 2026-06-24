# Contract Evidence: implement-module-workbench

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic prompt-rendering coverage for `rms.implement@v1`.

Executable coverage:

- `implement_prompt_classifies_change_before_steps` verifies the prompt asks for accepted intent and rationale before coding, scope expansion or module split decisions, change classification, contract or manifest updates before code changes, semantic implementation roles before file-level code, and concrete implementation guidance without claiming edits were made.
- `implement_prompt_routes_composite_parent_to_child` verifies that a composite parent prompt includes route evidence, recommends the domain child for rule/transition work, and warns not to add private implementation behavior to the composite parent.
- `workbench_run_record_writes_prompt_request_and_checks` verifies advisory workbench prompts can write deterministic run records without provider execution.

Provider execution uses the same rendered prompt and stores provider output under the generated run record.

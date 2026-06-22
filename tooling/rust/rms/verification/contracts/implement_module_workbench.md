# Contract Evidence: implement-module-workbench

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic prompt-rendering coverage for `rms.implement@v1`.

Executable coverage:

- `implement_prompt_classifies_change_before_steps` verifies the prompt asks for change classification, contract or manifest updates before code changes, and concrete implementation guidance without claiming edits were made.
- `workbench_run_record_writes_prompt_request_and_checks` verifies advisory workbench prompts can write deterministic run records without provider execution.

Provider execution uses the same rendered prompt and stores provider output under the generated run record.

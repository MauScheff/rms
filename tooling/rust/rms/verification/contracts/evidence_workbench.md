# Contract Evidence: evidence-workbench

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic prompt-rendering coverage for `rms.evidence@v1`.

Executable coverage:

- `evidence_prompt_names_smallest_proof` verifies the prompt asks for the smallest strong evidence and names manifest or implementation binding references to update.
- `workbench_run_record_writes_prompt_request_and_checks` verifies advisory workbench prompts can write deterministic run records without provider execution.

Provider execution uses the same rendered prompt and stores provider output under the generated run record.

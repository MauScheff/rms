# Contract Evidence: evolve-contract-workbench

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic prompt-rendering coverage for `rms.evolve-contract@v1`.

Executable coverage:

- `evolve_contract_prompt_classifies_compatibility` verifies the prompt asks for compatibility classification, migration and deprecation planning, and `rms check-compat` evidence.
- `workbench_run_record_writes_prompt_request_and_checks` verifies advisory workbench prompts can write deterministic run records without provider execution.

Provider execution uses the same rendered prompt and stores provider output under the generated run record.

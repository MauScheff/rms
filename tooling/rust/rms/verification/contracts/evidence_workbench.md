# Contract Evidence: evidence-workbench

Covered by `cargo test --manifest-path Cargo.toml`, including deterministic prompt-rendering coverage for `rms.evidence@v1`.

Executable coverage:

- `evidence_prompt_names_smallest_proof` verifies the prompt asks for the smallest strong evidence, names manifest or implementation binding references to update, includes transition-record or golden-timeline evidence where applicable, and includes the scaffold-evidence completion gate.
- `evidence_prompt_routes_rule_task_to_domain_proof` verifies routed domain work recommends transition, constructor, property, accepted/rejected, transition-record, replay-bundle, and first-bad-transition evidence.
- `evidence_prompt_routes_cli_task_to_boundary_proof` verifies routed boundary work recommends malformed-input, parser-to-domain-command, and command-envelope evidence.
- `evidence_prompt_names_parent_export_when_public_behavior_changes` verifies public behavior changes name parent/export contract evidence.
- `workbench_run_record_writes_prompt_request_and_checks` verifies advisory workbench prompts can write deterministic run records without provider execution.

Provider execution uses the same rendered prompt and stores provider output under the generated run record.

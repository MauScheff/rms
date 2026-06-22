# Contract Evidence: render-workbench-prompt

Covered by CLI smoke execution of `rms prompt <kind> <module> --task "<task>"`. The rendered prompt names its prompt id, advisory mode, bounded module context, workflow instructions, expected output, and deterministic checks.

Run-record behavior is covered by `rms prompt <kind> <module> --task "<task>" --record`, which writes `request.yaml`, `prompt.md`, and `checks.json` without calling a provider.

Executable coverage:

- `prompt_options_use_configured_ai_defaults` verifies `--ai` resolves the configured provider, Codex model, sandbox, and run-record directory from `.rms/config.yaml`.
- `prompt_options_require_configured_ai_provider` verifies `--ai` fails when no non-none default provider is configured.
- `workbench_run_record_writes_prompt_request_and_checks` verifies record creation remains deterministic and provider-independent.
- `evolve_contract_prompt_classifies_compatibility` verifies the contract-evolution prompt names compatibility and migration obligations.
- `evidence_prompt_names_smallest_proof` verifies the evidence prompt asks for focused proof and artifact references.

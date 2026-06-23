# Contract Evidence: render-workbench-prompt

Covered by CLI smoke execution of `rms prompt <kind> <module> --task "<task>"`. The rendered prompt names its prompt id, advisory mode, bounded module context, workflow instructions, expected output, and deterministic checks.

Run-record behavior is covered by `rms prompt <kind> <module> --task "<task>" --record`, which writes `request.yaml`, `prompt.md`, and `checks.json` without calling a provider.

Executable coverage:

- `prompt_options_use_configured_ai_defaults` verifies `--ai` resolves the configured provider, Codex model, sandbox, and run-record directory from `.rms/config.yaml`.
- `prompt_options_default_workspace_write_to_module_scope` verifies configured Codex `workspace-write` defaults to module write scope.
- `prompt_options_allow_configured_root_write_scope` verifies config can deliberately widen Codex `workspace-write` execution to repository-root scope.
- `prompt_options_require_configured_ai_provider` verifies `--ai` fails when no non-none default provider is configured.
- `workbench_run_record_writes_prompt_request_and_checks` verifies record creation remains deterministic and provider-independent while recording sandbox, write scope, and execution root.
- `provider_module_write_scope_uses_module_execution_root` verifies Codex `workspace-write` module scope selects the target module directory and appends explicit write-boundary instructions to the provider prompt.
- `provider_response_path_is_absolute_for_module_cd` verifies provider response capture remains rooted at the run record even when Codex is executed from a module directory.
- `evolve_contract_prompt_classifies_compatibility` verifies the contract-evolution prompt names compatibility and migration obligations.
- `evidence_prompt_names_smallest_proof` verifies the evidence prompt asks for focused proof and artifact references.

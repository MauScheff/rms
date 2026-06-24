# Contract Evidence: render-workbench-prompt

Covered by CLI smoke execution of `rms prompt <kind> <module> --task "<task>"`. The rendered prompt names its prompt id, advisory mode, bounded module context, workflow instructions, expected output, and deterministic checks.

Run-record behavior is covered by `rms prompt <kind> <module> --task "<task>" --record`, which writes `request.yaml`, `prompt.md`, and `checks.json` without calling a provider.

Executable coverage:

- `prompt_options_use_configured_ai_defaults` verifies `--ai` resolves the configured provider, Codex model, sandbox, provider timeout, and run-record directory from `.rms/config.yaml`.
- `prompt_options_reject_zero_provider_timeout` verifies configured provider timeouts must be positive.
- `prompt_options_default_workspace_write_to_module_scope` verifies configured Codex `workspace-write` defaults to module write scope.
- `prompt_options_allow_configured_root_write_scope` verifies config can deliberately widen Codex `workspace-write` execution to repository-root scope.
- `prompt_options_require_configured_ai_provider` verifies `--ai` fails when no non-none default provider is configured.
- `workbench_run_record_writes_prompt_request_and_checks` verifies record creation remains deterministic and provider-independent while recording sandbox, write scope, provider timeout, and execution root.
- `provider_module_write_scope_uses_module_execution_root` verifies Codex `workspace-write` module scope selects the target module directory and appends explicit write-boundary and timeout instructions to the provider prompt.
- `provider_response_path_is_absolute_for_module_cd` verifies provider response capture remains rooted at the run record even when Codex is executed from a module directory.
- `provider_wait_times_out_and_terminates_child` verifies provider execution is terminated after the selected timeout.
- `intent_prompt_gates_implementation_on_accepted_context` verifies the intent-capture prompt names the think-before-code gate and the canonical artifacts to update before implementation.
- `evolve_contract_prompt_classifies_compatibility` verifies the contract-evolution prompt names compatibility and migration obligations.
- `evidence_prompt_names_smallest_proof` verifies the evidence prompt asks for focused proof and artifact references.

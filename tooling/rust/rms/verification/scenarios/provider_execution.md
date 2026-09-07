# Scenario Evidence: provider-execution

Provider-backed workbench execution is opt-in with `--provider codex` or with `--ai` when `.rms/config.yaml` declares a non-none default provider. The CLI renders the same bounded prompt used by advisory mode, writes a run record, invokes `codex exec` with stdin and `--output-last-message`, passes the configured model and reasoning effort explicitly, waits only for the configured provider timeout, terminates a timed-out provider process, and stores provider stdout, stderr, final response, model, and reasoning effort in the run directory.

`provider_binding_preferences_are_closed_before_design` verifies that provider rationale prose cannot become a scaffold binding, that an exact supported language survives normalization, and that the design action receives only the supported binding identifier.

This scenario is not run by default in CI because it requires an authenticated provider command. Deterministic validation and advisory prompt rendering remain provider-independent.

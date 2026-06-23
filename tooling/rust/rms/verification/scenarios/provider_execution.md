# Scenario Evidence: provider-execution

Provider-backed workbench execution is opt-in with `--provider codex` or with `--ai` when `.rms/config.yaml` declares a non-none default provider. The CLI renders the same bounded prompt used by advisory mode, writes a run record, invokes `codex exec` with stdin and `--output-last-message`, waits only for the configured provider timeout, terminates a timed-out provider process, and stores provider stdout, stderr, and final response in the run directory.

This scenario is not run by default in CI because it requires an authenticated provider command. Deterministic validation and advisory prompt rendering remain provider-independent.

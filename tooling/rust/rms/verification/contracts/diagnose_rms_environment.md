# Contract Evidence: diagnose-rms-environment

Covered by CLI execution in the repository root. The command reports discovered RMS artifacts, validation status, optional workbench config status, run-record readiness, native tool availability, and optional provider command availability without mutating project artifacts.

Executable coverage:

- `diagnose_report_includes_config_and_serializes_to_json` verifies `.rms/config.yaml` readiness, including provider timeout, is represented in the shared diagnose report and serializes for `rms diagnose --json`.
- The same report includes `git source revision` readiness and guidance when strict audit cannot yet be used as production evidence.
- Repository smoke execution of `rms diagnose --root .` checks the text report path.

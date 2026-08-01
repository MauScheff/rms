# Contract Evidence: diagnose-rms-environment

Covered by CLI execution in the repository root. The command reports discovered RMS artifacts, validation status, optional workbench config status, run-record readiness, native tool availability, and optional provider readiness without mutating project artifacts. For configured Codex routing it resolves the effective model from RMS project configuration before Codex user configuration, verifies structured-output support, and checks that model against the catalog bundled with the installed Codex binary. It never silently selects or downgrades a model.

Executable coverage:

- `diagnose_report_includes_config_and_serializes_to_json` verifies `.rms/config.yaml` readiness, including provider timeout, is represented in the shared diagnose report and serializes for `rms diagnose --json`.
- `codex_readiness_checks_the_effective_model_without_silently_selecting_one` verifies compatible project and user model sources, an absent bundled model, and an older Codex binary that cannot expose its bundled catalog. Incompatibility prescribes `codex update` first and explicit `ai.codex.model` pinning as the alternative.
- The same report includes `git source revision` readiness and guidance when strict audit cannot yet be used as production evidence.
- Repository smoke execution of `rms diagnose --root .` checks the text report path.

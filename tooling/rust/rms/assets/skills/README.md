# RMS Agent Skills

These skills express architecture workflows without assuming a programming language or coding-agent vendor.

The RMS CLI is the stable workbench for humans and agents. Skills should make agents use the CLI before carrying RMS rules in prompt memory. The CLI inspects canonical artifacts, builds bounded context, runs deterministic checks, and records evidence; skills only choose the right workflow.

Canonical skills:

- `inspect-module`
- `implement-change`
- `refactor-module`
- `prune-module`
- `add-module`
- `evolve-contract`
- `compose-modules`
- `verify-module`

Each skill uses the portable `SKILL.md` form with only common `name` and `description` frontmatter. Agent-specific packaging belongs under `integrations/` or generated installation directories.

The semantic workflow in these skills is normative only where it restates `SPEC.md`. The skills themselves are operational guidance.

## CLI-first workflow

Use these commands when available:

```text
rms diagnose
rms diagnose --json
rms config init
rms explain <module> [question]
rms explain "question" --root <module-directory>
rms plan <module> --task "<task>"
rms implement <module> --task "<task>"
rms evolve-contract <module> --task "<task>"
rms evidence <module> --task "<task>"
rms refactor <module> --task "<task>"
rms review <module> [--diff <git-spec>]
rms prompt <kind> <module> --task "<task>"
rms prompt <kind> <module> --task "<task>" --record
rms prompt <kind> <module> --task "<task>" --provider codex
rms prompt <kind> <module> --task "<task>" --ai
rms run list
rms run latest
rms run inspect <run-id-or-path>
rms inspect <module>
rms context <module> --task "<task>"
rms validate --root <root>
rms compose --root <root>
rms check-compat <old-module> <new-module>
rms verify <implementation.yaml>
rms conformance <module> --implementation <implementation.yaml>
rms release check --root <root>
```

Skills should use `rms explain`, `rms implement`, `rms evolve-contract`, `rms evidence`, `rms refactor`, `rms prompt <kind>`, and the advisory `rms plan` / `rms review` commands to render bounded prompts for humans or agents. Use `--ai` only when `.rms/config.yaml` declares the intended default provider; use `--provider codex` for an explicit Codex run.

## Safety

The canonical skills are instruction-only. Agent-specific packages may add scripts, but executable additions should be version-pinned, reviewed, and granted least privilege. Skills should call project-native RMS validation rather than embedding a second set of architectural rules.

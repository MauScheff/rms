# RMS Agent Skills

These skills express architecture workflows without assuming a programming language or coding-agent vendor.

The RMS CLI is the stable workbench for humans and agents. Skills should make agents use the CLI before carrying RMS rules in prompt memory. The CLI inspects canonical artifacts, builds bounded context, runs deterministic checks, and records evidence; skills only choose the right workflow.

Product intent is enough input from the user. Agents should convert natural language into RMS semantics by asking only necessary clarifying questions, surfacing edge cases, naming what must never happen, and applying semantic changes before code.

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
rms spec plan <module.yaml|implementation.yaml> --task "<task>"
rms spec apply <module.yaml|implementation.yaml> --change-json '<json>'
rms spec apply <module.yaml|implementation.yaml> --change-yaml '<yaml>'
rms spec check <module.yaml|implementation.yaml>
rms spec diff <module.yaml|implementation.yaml>
rms machine plan <implementation.yaml> --task "<task>"
rms machine apply <implementation.yaml> --change-json '<json>'
rms machine apply <implementation.yaml> --change-yaml '<yaml>'
rms machine check <implementation.yaml>
rms machine diff <implementation.yaml>
rms trace check <trace-bundle>
rms trace replay <trace-bundle>
rms trace diagnose <trace-bundle>
rms inspect <module>
rms context <module> --task "<task>"
rms validate --root <root>
rms compose --root <root>
rms check-compat <old-module> <new-module>
rms verify <implementation.yaml|composite-module.yaml>
rms conformance <module> --implementation <implementation.yaml>
rms release check --root <root>
```

Skills should use `rms explain`, `rms implement`, `rms evolve-contract`, `rms evidence`, `rms refactor`, `rms prompt <kind>`, and the advisory `rms plan` / `rms review` commands to render bounded prompts for humans or agents. Use `rms spec plan/apply/check` when a change needs new laws, contracts, states, commands, events, effects, effect results, replies, rejections, transitions, semantic roles, public entrypoints, or evidence obligations. `rms spec apply` records the exact applied semantic-change object under `verification/changes/`; command logs with placeholders are not evidence. Use `rms machine plan/apply/check` for focused inner-machine edits after semantic obligations are already correct. For external truth, decide what happens when an outcome is unknown, duplicate, stale, partial, conflicting, delayed, or later corrected; use reconciliation or recovery evidence when correctness depends on that behavior. Use `--ai` only when `.rms/config.yaml` declares the intended default provider; use `--provider codex` for an explicit Codex run. Provider runs are bounded by `ai.codex.timeout_seconds` or `--provider-timeout-seconds`.

## Safety

The canonical skills are instruction-only. Agent-specific packages may add scripts, but executable additions should be version-pinned, reviewed, and granted least privilege. Skills should call project-native RMS validation rather than embedding a second set of architectural rules.

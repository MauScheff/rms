# Codex Integration

**Status:** Non-normative adapter guidance  
**Checked against official OpenAI documentation:** 2026-06-20

RMS remains agent-neutral. This adapter makes the canonical CLI, guidance, and skills convenient in Codex without defining a second architecture.

## Repository Instructions

Keep the portable agreement in root `AGENTS.md`. Generate or refresh it from the RMS CLI; put task-specific mechanics in repository skills rather than permanent startup context.

The onboarding order is:

```text
init → authorized bootstrap commit → design → recommended scaffold
```

For ordinary work, Codex needs only the five-command doorway:

```bash
rms check --environment --root .
rms next "<intent>" --root .
rms explain "<question>" --root .
rms check --root .
rms view --root .
```

`next` selects the owner, task lane, context, skill, and proof path. Codex should use its compact response first and request `--details` only when needed. `rms help --all` exposes specialist commands when the selected skill prescribes one.

Production completion is focused proof → `rms check --changes` → authorized candidate commit → `rms check --committed`.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness. Report the applicable pending state instead of executing or implying a commit.

## Skills

RMS skills use the Agent Skills `SKILL.md` format. Project skills live under:

```text
.agents/skills/<skill-name>/SKILL.md
```

The canonical source remains repository `skills/`. Generated project, embedded, and plugin copies must match it. Skills call the shared CLI and carry focused declaration and proof mechanics; they do not redefine manifests or contracts.

`rms check --environment --json` reports observed skill sources. A detected project copy, user installation, marketplace entry, or plugin cache is not proof that Codex injected it into the current task. RMS reports `runtime_activation: unknown` and `precedence: host-defined`; the current injected Codex skill catalog is the runtime authority.

For agent automation, use `--json`. The `rms.surface/v2` envelope exposes typed `program` plus `args` for executable actions. A `kind: manual` action with `authorization: host-required` is never executable by inference.

Provider-backed prompts remain explicit specialist workflows. They are advisory until canonical apply and deterministic checks succeed. Review and pin executable skill and plugin content before installation.

## Plugin Wrapper

The optional plugin wrapper lives at:

```text
integrations/codex/rms
```

Refresh all managed RMS skill distributions through its compatibility entrypoint:

```bash
./integrations/codex/rms/scripts/sync-skills.sh
```

The wrapper calls the canonical repository sync script. It must preserve unrelated local skills and remain packaging rather than semantic authority.

## Hooks

Hooks may call shared RMS checks at lifecycle boundaries. They should use the same `rms check` mode as CI and must not duplicate semantic rules, invoke providers implicitly, or grant Git authority.

## Recommended Layout

```text
AGENTS.md                     Concise portable guidance
skills/                       Canonical skill source
.agents/skills/               Managed project copies
integrations/codex/rms/       Optional plugin packaging
```

## Official References

- [Custom instructions with AGENTS.md](https://developers.openai.com/codex/guides/agents-md)
- [Agent Skills](https://developers.openai.com/codex/skills)
- [Plugins](https://developers.openai.com/codex/plugins)
- [Hooks](https://developers.openai.com/codex/hooks)

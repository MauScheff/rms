# Codex Integration

**Status:** Non-normative adapter guidance  
**Checked against official OpenAI documentation:** 2026-06-20

RMS remains agent-neutral. This adapter makes the neutral manifests and skills convenient in Codex.

## Repository instructions

Keep the portable working agreement in the repository root as `AGENTS.md`. Codex discovers `AGENTS.md` from the project root down toward the working directory, allowing more local instructions for nested modules.

Keep `AGENTS.md` concise. Detailed, task-specific procedures belong in Agent Skills rather than permanent startup context.

## Skills

Codex skills build on the open Agent Skills standard. RMS skills use a `SKILL.md` entry point with `name` and `description` frontmatter. To make the canonical skills available to Codex, install or generate them under:

```text
.agents/skills/<skill-name>/SKILL.md
```

Codex can discover repository skills from `.agents/skills` directories between the working directory and repository root.

The canonical source should remain the project-level `skills/` directory. Review and pin executable skill content before installation. A release tool should copy or generate the Codex installation directory to avoid hand-maintained duplication.

## Plugins

Package the skills as a Codex plugin only when installable distribution is useful. Plugins can bundle skills and integrations. Plugin packaging is an adapter concern and must not become the source of RMS semantics.

This repository includes a thin Codex plugin wrapper:

```text
integrations/codex/rms
```

The wrapper packages the canonical RMS skills for Codex. Refresh the packaged copy before release:

```bash
./integrations/codex/rms/scripts/sync-skills.sh
```

Install the neutral CLI separately:

```bash
cargo install --path tooling/rust/rms
```

## Hooks

Codex hooks may invoke shared RMS validation at lifecycle points, such as when a turn stops. Hooks should call the same repository validator used by CI; they should not implement separate architectural rules.

## Recommended layout

```text
AGENTS.md
skills/                         Canonical skill source
.agents/skills/                 Generated or installed Codex skills
integrations/codex/             Optional plugin and hook packaging
```

## Official references

- [Custom instructions with AGENTS.md](https://developers.openai.com/codex/guides/agents-md)
- [Agent Skills](https://developers.openai.com/codex/skills)
- [Plugins](https://developers.openai.com/codex/plugins)
- [Hooks](https://developers.openai.com/codex/hooks)

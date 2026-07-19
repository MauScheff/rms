# Claude Code Integration

**Status:** Non-normative adapter guidance  
**Checked against official Anthropic documentation:** 2026-06-20

RMS remains agent-neutral. Claude Code uses the same canonical CLI, guidance, and skills as every other agent.

## Repository Instructions

Claude Code reads `CLAUDE.md`. Keep it minimal and import the portable agreement:

```md
@AGENTS.md
```

The five-command doorway is:

```bash
rms check --environment --root .
rms next "<exact user task>" --root . --ai
rms explain "<question>" --root .
rms check --root .
rms view --root .
```

Claude extracts typed facts without topology; recorded read-only `--ai` extraction is the alternative. Use the compact result first, `--details` for complete canonical evidence, `--json` for the typed `rms.surface/v2` envelope, and `rms help --all` only when a selected skill prescribes specialist work.

Finish through focused proof → `rms check --changes` → authorized candidate commit → `rms check --committed`. Git commits are required evidence, not implied authority; a manual `host-required` action in JSON does not authorize Claude to execute Git.

## Skills

Project skills live under:

```text
.claude/skills/<skill-name>/SKILL.md
```

Canonical skill content remains in repository `skills/`. Managed Claude copies call the shared CLI and must not redefine semantic artifacts.

Detected copies are observable evidence only. RMS cannot inspect the current Claude task's injected catalog, so runtime activation is unknown and precedence is host-defined.

Provider-backed work is explicit and specialist-only. A provider response remains advisory until canonical apply and deterministic checks succeed.

## Plugins and Hooks

A Claude plugin may package skills, agents, hooks, and MCP servers, but packaging must not become semantic authority. Hooks may invoke the same `rms check` modes as local work and CI; they should not implement separate architectural rules.

## Recommended Layout

```text
AGENTS.md                     Portable instructions
CLAUDE.md                     Imports AGENTS.md
skills/                       Canonical skill source
.claude/skills/               Managed Claude copies
integrations/claude-code/     Optional packaging
```

## Official References

- [Project memory and CLAUDE.md](https://code.claude.com/docs/en/memory)
- [Skills](https://code.claude.com/docs/en/skills)
- [Hooks](https://code.claude.com/docs/en/hooks-guide)
- [Plugins](https://code.claude.com/docs/en/plugins)

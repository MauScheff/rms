# Claude Code Integration

**Status:** Non-normative adapter guidance  
**Checked against official Anthropic documentation:** 2026-06-20

RMS remains agent-neutral. This adapter makes the neutral manifests and skills convenient in Claude Code.

## Repository instructions

Claude Code reads `CLAUDE.md`, not `AGENTS.md` directly. Use a minimal root `CLAUDE.md` that imports the portable instructions:

```md
@AGENTS.md
```

Claude Code treats these files as context, not deterministic enforcement. Keep them concise and use hooks or CI for rules that must always run.

## Skills

Claude Code skills follow the open Agent Skills standard, with Claude-specific extensions available when needed. To make RMS skills available as project skills, install or generate them under:

```text
.claude/skills/<skill-name>/SKILL.md
```

The canonical source should remain the project-level `skills/` directory. Review and pin executable skill content before installation. Generate or copy the Claude Code installation directory rather than maintaining two divergent skill definitions.

## Plugins

Package the RMS skills as a Claude Code plugin when marketplace or organization-wide distribution is useful. A plugin may include skills, agents, hooks, and MCP servers. Plugin packaging must not redefine the semantic manifests or contracts.

## Hooks

Claude Code hooks provide deterministic lifecycle actions. Use them to invoke shared RMS validation, formatting, or permission checks. The hook should call the same command used by CI.

## Recommended layout

```text
AGENTS.md
CLAUDE.md                     Imports AGENTS.md
skills/                       Canonical skill source
.claude/skills/               Generated or installed Claude skills
integrations/claude-code/     Optional plugin and hook packaging
```

## Official references

- [Project memory and CLAUDE.md](https://code.claude.com/docs/en/memory)
- [Skills](https://code.claude.com/docs/en/skills)
- [Hooks](https://code.claude.com/docs/en/hooks-guide)
- [Plugins](https://code.claude.com/docs/en/plugins)

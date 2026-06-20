# Agent Integrations

RMS is agent-neutral. The canonical architectural workflows live in `../skills/`; these files explain how to install or package them for current coding agents.

- `CODEX.md` covers `AGENTS.md`, `.agents/skills`, plugins, and hooks.
- `CLAUDE_CODE.md` covers `CLAUDE.md`, `.claude/skills`, plugins, and hooks.
- `GENERIC_AGENT.md` defines the minimum adapter behavior for any other agent.

Vendor integrations are versioned independently and may be regenerated. They must never become the only source of module semantics.

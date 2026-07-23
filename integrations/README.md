# Agent Integrations

RMS is agent-neutral. The canonical architectural workflows live in `../skills/`; these files explain how to install or package them for current coding agents.

Every adapter starts software-change work through `rms next "<exact change task>"`, uses `rms explain` for canonical questions, and completes through `rms check`; specialist commands remain discoverable through `rms help --all` and selected skills. Read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, and discussion that requests no change stay in native project tools. If they reveal a proposed change, the adapter stops before editing and routes that exact change task through RMS.

- `CODEX.md` covers `AGENTS.md`, `.agents/skills`, plugins, and hooks.
- `CLAUDE_CODE.md` covers `CLAUDE.md`, `.claude/skills`, plugins, and hooks.
- `GENERIC_AGENT.md` defines the minimum adapter behavior for any other agent.

Vendor integrations are versioned independently and may be regenerated. They must never become the only source of module semantics.

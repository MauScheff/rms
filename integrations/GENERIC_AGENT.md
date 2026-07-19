# Generic Coding-Agent Integration

A coding agent needs no native RMS integration. It needs the CLI, concise repository guidance, and a way to load the task-selected skill.

## Doorway

```bash
rms check --environment --root .
rms next "<exact user task>" --root . --ai
rms explain "<question>" --root .
rms check --root .
rms view --root .
```

Use `rms help --all` only when the selected skill or detailed response prescribes a specialist command.

The agent extracts typed semantic facts from the user's words without proposing modules, shapes, or topology. It may instead opt into recorded read-only `--ai` extraction. The agent should then follow this loop:

1. ask `next` for the owner, lane, context, and immediate action;
2. use `explain` when canonical meaning is unclear;
3. load the selected repository skill;
4. apply semantic declarations before implementation and edit only declared roles;
5. run focused proof and the prescribed `check` modes.

Production completion is focused proof → `rms check --changes` → authorized candidate commit → `rms check --committed`. Git commits are required evidence, not implied authority; the agent acts only when task and host policy grant Git authority.

## Agent JSON

Use `--json` for automation. Every primary response has `schema: rms.surface/v2`, command, result, summary, reasons, warnings, next action, done conditions, and a details-availability flag. `next` also carries lane, confidence, owner state, and ordered typed steps; `explain` carries its answer and evidence paths; `check` carries its mode and constituent summaries. `--details --json` nests complete evidence under that same envelope.

An executable action has:

```yaml
kind: command
program: rms
args: [check, --root, .]
display: rms check --root .
authorization: none
```

Execute `program` with the argument array directly; never parse `display` as shell input.

A manual action has an instruction rather than `program` and `args`. If `authorization` is `host-required`, stop unless the user and host policy already grant that authority. Candidate commits are always manual.

## Context and Authority

When needed, provide the agent with the system summary, selected module, applicable glossary entries, public and dependency contracts, relevant decisions, declared roles, and focused verification commands. Prefer the context paths emitted by `next --details` over a hand-maintained packet.

Canonical RMS artifacts and deterministic checks remain authoritative over conversation, reports, and generated guidance. Skills select workflows but do not define semantics.

Skill-source detection does not prove runtime activation. The agent host's injected catalog is authoritative; RMS reports activation as unknown and precedence as host-defined.

Provider execution, filesystem writes, credentials, Git operations, and external publication remain explicit host-authorized capabilities. Do not make model-specific prompting part of the semantic specification.

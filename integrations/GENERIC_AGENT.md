# Generic Coding-Agent Integration

A coding agent does not need native RMS support to work reliably with an RMS project. It needs access to the `rms` CLI and concise repository instructions.

Start with:

```bash
rms diagnose
rms diagnose --json
rms config init
rms explain <module>
rms explain "question" --root <module-directory>
rms plan <module> --task "<task>"
rms implement <module> --task "<task>"
rms evolve-contract <module> --task "<task>"
rms evidence <module> --task "<task>"
rms refactor <module> --task "<task>"
rms prompt <kind> <module> --task "<task>" --ai
rms context <module> --task "<task>"
rms review <module>
rms prompt <kind> <module> --task "<task>" --record
rms run list
rms run latest
rms run inspect <run-id-or-path>
rms release check --root <root>
```

Provide the agent with a task context packet containing:

```text
System summary
Target module manifest
Applicable glossary entries
Public contracts
Direct dependency contracts
Relevant decisions
Verification commands
```

The agent should follow the workflows in `skills/` and the portable rules in `AGENTS.md`, but the CLI is the preferred operating surface. Skills and prompts should not duplicate RMS rules when a CLI command can inspect, render a workbench prompt, or validate the canonical artifacts.

When preserving an agent interaction matters, use the CLI run-record options rather than an agent-specific transcript format.

A generic adapter should support:

```text
Loading concise repository instructions
Invoking Agent Skills or equivalent workflows
Running the neutral validator
Restricting filesystem, network, and credential access
Returning verification evidence
```

Do not make model-specific prompting part of the semantic specification. Prompt adaptations should remain replaceable.

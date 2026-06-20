# Generic Coding-Agent Integration

A coding agent does not need native RMS support to work reliably with an RMS project.

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

The agent should follow the workflows in `skills/` and the portable rules in `AGENTS.md`.

A generic adapter should support:

```text
Loading concise repository instructions
Invoking Agent Skills or equivalent workflows
Running the neutral validator
Restricting filesystem, network, and credential access
Returning verification evidence
```

Do not make model-specific prompting part of the semantic specification. Prompt adaptations should remain replaceable.

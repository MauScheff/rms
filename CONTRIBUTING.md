# Contributing

RMS is intentionally small. Contributions should improve clarity, enforceability, or demonstrated usefulness rather than add architectural vocabulary by default.

## Types of change

- **Semantic core:** modules, ownership, contracts, invariants, effects, profiles, composition, substitutability, or conformance. Requires an RFC, compatibility analysis, migration guidance, and worked example.
- **Exchange format:** manifests, schemas, report formats, and package conventions. Must be versioned and validated against fixtures.
- **Language binding:** maps RMS semantics to a language or toolchain without redefining them.
- **Agent integration:** adapts instructions, skills, hooks, or plugins without becoming a source of semantic truth.
- **Editorial:** improves explanation without changing requirements.

## Pull-request evidence

A material change should include:

```text
Problem and intended outcome
Affected normative sections
Compatibility impact
Updated schemas and examples when applicable
Validation and link-check results
Migration guidance for breaking changes
```

## Scope test

A new core concept should be added only when it is semantically important, language-neutral, agent-neutral, observable or verifiable, useful across more than one implementation, and difficult to express as an extension or profile.

## Pilot rule

During the 0.1 pilot, prefer reports from real implementations over speculative expansion. Friction that appears in one language or agent should first be solved in its binding or integration layer.

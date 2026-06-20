# RMS Governance and Stability

RMS aims to be durable without becoming frozen. The 0.1 semantic core is frozen for pilot use; changes should be driven by observed implementation friction rather than speculative completeness. The core model should change rarely; schemas, bindings, and integrations should be able to improve independently.

## 1. Stability layers

### Stable semantic core

Changes to these concepts require strong justification and a major specification version when incompatible:

```text
Module ownership
Public/private boundaries
Contracts
Invariants
Effects
Operational semantics
Profiles
Conformance
```

### Evolvable exchange formats

These may evolve more frequently with explicit versioning:

```text
YAML field names
JSON Schemas
Conformance-report format
Context-packet format
Module-package layout
```

### Independent bindings

These version independently of the core:

```text
Language bindings
Framework bindings
Codex integration
Claude Code integration
Other agent integrations
IDE and CI plugins
```

A binding may add convenience but may not redefine core semantics.

## 2. Specification changes

A material change should include:

1. the problem being solved;
2. why existing extension mechanisms are insufficient;
3. compatibility impact;
4. migration guidance;
5. conformance changes;
6. at least one worked example.

Large changes should be proposed as an RFC before incorporation.

## 3. Extension policy

Projects may add `x-` fields to manifests. Widely adopted extensions may later become standard after demonstrating:

```text
Clear semantics
Use across more than one language or agent
Deterministic validation
Backward-compatible migration
Low conceptual overlap with existing fields
```

## 4. Deprecation

A field or requirement should be deprecated before removal whenever practical.

Deprecation documentation should state:

```text
Replacement
Reason
First deprecated version
Last supported version
Migration procedure
```

## 5. Conformance integrity

A release of the specification should publish:

```text
Normative specification
Schemas
Conformance fixtures
At least one valid example
At least one invalid example per important rule
Change log
```

No vendor integration may claim stronger authority than the conformance suite.

## 6. Scope discipline

The project should resist adding concepts merely because one language, framework, or agent supports them.

A proposal belongs in the core only when it is:

```text
Semantically important
Broadly applicable
Language-neutral
Agent-neutral
Observable or verifiable
Difficult to express as an extension or profile
```

## 7. Open-source project decisions

Before a public release, maintainers should explicitly choose:

- project name and namespace availability;
- software and documentation license;
- trademark policy, if any;
- security-reporting process;
- maintainer and decision model;
- release cadence and support policy;
- contributor certificate or developer-certificate policy, if desired.

These governance decisions are separate from the architectural method and should not be embedded in the semantic specification.

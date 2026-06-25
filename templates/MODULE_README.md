# <Module Name>

## Purpose

One sentence describing why the module exists.

## Owner and boundary

```text
Owning context:
Owns:
Does not own:
```

## Public surface

```text
Provides:
Requires:
```

## Invariants

```text
- <Invariant identifier>: <statement>
```

## Representation and state updates

```text
Semantic state:
Derived facts:
Commands/events that change state:
Mutation policy:
```

State whether semantic representations are immutable by default, where transition functions live, and which localized mutable runtime structures are allowed behind ports or adapters.

## Profiles

```text
core
stateful      # when applicable
distributed   # when applicable
workflow      # when applicable
boundary      # when applicable
```

## Effects and operational semantics

Describe only the effects and semantics that matter to callers and operators.

## Compatibility

State the public compatibility, deprecation, and migration policy.

## Intent and rationale

Link only to durable records that explain current semantics.

```text
Intent:
Decisions:
Semantic traces:
```

## Verification

```text
Laws:
Contracts:
Scenarios:
Boundaries:
```

## Change protocols

Name the recurring changes that carry hidden collateral updates. Keep these derived from the module manifest, contracts, effects, state model, and evidence.

```text
- <protocol-id>
  Applies when:
  Affected surfaces:
  Required updates:
  Verify:
```

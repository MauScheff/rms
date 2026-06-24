# tic-tac-toe-rules

Purpose: Own Tic-Tac-Toe board rules, legal moves, turn order, and terminal outcomes
Kind: `library`
Implementation binding: `rust` via `implementation.yaml`.

## Profiles

- `core`
- `stateful`

## Semantic Shape

Shape: `Domain Engine`: `domain-engine` (pure decisions, closed variants, validated values, transitions, laws, and trace replay)

Required roles:
- `representation`
- `commands`
- `transitions`
- `trace-replay`
- `law-evidence`

Representation is the RMS-level role for closed variants, validated values, commands, states, events, and result/rejection types. Implement it with language-idiomatic files or modules; do not treat a folder named `domain` or `types` as canonical architecture.

## Representation Decisions

- Closed domain alternatives should use ADTs, sealed variants, enums, or tagged constructors.
- Public values with validity rules should use private fields, validated constructors, explicit failure types, semantic-function bindings, and evidence.
- Expected domain failures should be explicit result or rejection values rather than ambient exceptions.
- Lifecycle or order-dependent behavior should use a transition model with accepted and rejected outcomes.
- Boundary input should be parsed into domain commands before reaching pure decisions.
- Public read models or result structs produced only by queries/projectors may keep private fields without public constructors only when `implementation.yaml` declares them in `architecture.allowed_missing_constructors` and evidence names the producing query/projector.
- Do not add a fake public constructor only to satisfy a binding check; either expose a real contract-backed constructor or document the query-produced exception.

## Canonical Artifacts

- `module.yaml` is the source of module ownership, public surface, dependencies, effects, invariants, profiles, and compatibility.
- `contracts/` contains public RMS contracts only: commands, queries, events, APIs, capabilities, schemas, and externally consumed failure semantics.
- `implementation.yaml`, when present, binds code symbols to contracts, invariants, assumptions, and evidence.
- `verification/` contains evidence for declared promises. Evidence should name the source revision and command or tool used.

## Before Changing Behavior

1. Fill `module.yaml` with owned concepts, data, decisions, public surface, dependencies, effects, invariants, and verification references that are true for this module.
2. Add or update public contracts before implementing externally consumed behavior.
3. Keep private implementation details out of `contracts/` unless consumers depend on them.
4. Add the smallest evidence that proves the declared promise, including negative cases for invalid inputs or illegal transitions when applicable.
5. Run `rms validate --root <system-root>` and `rms compose --root <system-root>`; run `rms verify implementation.yaml` when an implementation binding exists.

## Agent Workflow

Use `rms design --root <system-root> --task "<task>"` when module boundaries or semantic shapes are unclear. Use `rms explain module.yaml` and `rms context module.yaml --task "<task>"` before implementation work. Use `rms evolve-contract module.yaml --task "<task>"` when public meaning changes, and `rms evidence module.yaml --task "<task>"` when proof design is unclear.

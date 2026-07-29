# example-python

Purpose: Demonstrate Python module scaffolding.
Kind: `library`
Implementation binding: `python` via `implementation.yaml`.

## Profiles

- `core`

## Semantic Shape

Shape: `Domain Engine`: `domain-engine` (pure decisions, closed variants, validated values, transition records, laws, and replay bundles)

Required roles:
- `representation`
- `commands`
- `transitions`
- `trace-replay`
- `law-evidence`

Representation is the RMS-level role for closed variants, validated values, commands, states, events, and result/rejection types. Implement it with language-idiomatic files or modules; do not treat a folder named `domain` or `types` as canonical architecture. Traceable-machine roles make debugging bad states explicit: message envelopes carry identity and causality, transitions return next state plus emitted events, commands, effects, and reply, transition records capture before/after/input/output/source provenance, journals explain, replay bundles reproduce, and first-bad-transition evidence points to the fix. Use domain-named role suffixes where the language allows it: `<Domain>Machine`, `<Domain>State`, `<Domain>Command`, `<Domain>Event`, `<Domain>Effect`, `<Domain>EffectResult`, `<Domain>Reply`, `<Domain>Rejection`, `<Domain>Transition`, and `<Domain>TransitionRecord`. Do not derive inner role names from role or surface suffixes such as rules, engine, adapter, cli, web, rust, swift, or js unless those words are genuine domain language.

## Representation Decisions

- Closed domain alternatives should use ADTs, sealed variants, enums, or tagged constructors.
- Public values with validity rules should use private fields, validated constructors, explicit failure types, semantic-function bindings, and evidence.
- Validated numeric values should use checked, saturating, bounded, or explicitly proven arithmetic, with evidence for overflow, floors, ceilings, and rounding when arithmetic affects decisions.
- Expected domain failures should be explicit result or rejection values rather than ambient exceptions.
- Lifecycle or order-dependent behavior should use a transition model with accepted and rejected outcomes, transition records, replay bundles, and first-bad-transition diagnostics when applicable.
- Boundary input should be parsed into enveloped domain commands before reaching pure decisions.
- Runnable surfaces adapt outside input into declared RMS commands, may render or execute declared boundary effects, and must not reimplement domain decisions or call private module internals.
- Public read models or result structs produced only by queries/projectors may keep private fields without public constructors only when `implementation.yaml` declares them in `architecture.allowed_missing_constructors` and evidence names the producing query/projector.
- Projections observe and derive timelines; they do not emit workflow commands or mutate another module's state.
- Do not add a fake public constructor only to satisfy a binding check; either expose a real contract-backed constructor or document the query-produced exception.

## Runtime Monitor Decisions

- Use this section when the module declares the `monitor` profile or `runtime-monitor` shape.
- Declare observed inputs, derived facts or streams, trigger conditions, monitor authority, retrigger/idempotency policy, and fail-open/fail-closed/degraded behavior in `module.yaml`.
- Supervisory outputs must be public commands, events, alarms, findings, or capabilities. Do not mutate controlled module state directly.
- Add runtime evidence for trigger and non-trigger cases before relying on a monitor for release or operational assurance.

## Canonical Artifacts

- `module.yaml` is the source of module ownership, public surface, dependencies, effects, invariants, profiles, and compatibility.
- `contracts/` contains public RMS contracts only: commands, queries, events, APIs, capabilities, schemas, and externally consumed failure semantics.
- `implementation.yaml`, when present, binds code symbols to contracts, invariants, assumptions, and evidence.
- `verification/` contains evidence for declared promises. Evidence should name the source revision and command or tool used.

## Before Changing Behavior

1. Fill `module.yaml` with owned concepts, data, decisions, public surface, dependencies, effects, invariants, and verification references that are true for this module.
2. Add or update public contracts before implementing externally consumed behavior.
3. Keep private implementation details out of `contracts/` unless consumers depend on them.
4. Add the smallest evidence that proves the declared promise, including negative cases for invalid inputs, illegal transitions, replayed bad states, numeric boundary cases, or passive projections when applicable.
5. Use `rms spec apply module.yaml --change-yaml '<semantic-change>'` when new laws, contracts, states, commands, events, effects, effect results, replies, rejections, transitions, semantic roles, runnable surfaces, public entrypoints, or evidence obligations are needed; then fill declared role bodies.
6. Use `rms surface apply implementation.yaml --kind runnable-boundary --surface <surface> --entrypoint <path> --delegates-to <role-or-symbol> --command <public-command>` before adding or changing app, UI, CLI, browser, HTTP, batch, mobile, desktop, or executable entrypoints.
7. Use `rms machine apply implementation.yaml --change-yaml '<machine-change>'` only for focused inner-machine edits after laws, public contracts, and evidence obligations are already correct.
8. Run `rms validate --root <system-root>` and `rms compose --root <system-root>`; run `rms spec check module.yaml`, `rms machine check implementation.yaml`, `rms surface check implementation.yaml`, `rms structure implementation.yaml`, and `rms verify implementation.yaml` when an implementation binding exists.
9. Replace scaffold placeholder evidence before declaring this module implemented; `rms validate --root <system-root>` should not report placeholder, bootstrap, unpinned, or semantic-shape-only evidence for implemented promises.

## Quick Machine Probe

Use `rms probe implementation.yaml --describe`, then copy an advertised example into `rms probe implementation.yaml --input '<JSON>'`. Ordered scenario files can assert state and case paths. Probes call the real transition-record function without executing effects and remain ephemeral unless `--out` is supplied.

## Agent Workflow

Use `rms design --root <system-root> --task "<task>" --intent-yaml '<rms/intent-model/v0.1>'` when module boundaries or semantic shapes are unclear. Use `rms explain --module module.yaml` and `rms context module.yaml --task "<task>"` before implementation work. Use `rms spec plan module.yaml --task "<task>"` before changing product meaning, laws, contracts, runnable surfaces, effects, machine structure, or evidence obligations. Use `rms surface apply/check implementation.yaml` before app/UI/CLI/browser/HTTP/batch/mobile/desktop/executable entrypoint changes. Use `rms machine plan implementation.yaml --task "<task>"` only for focused inner-machine edits after the semantic layer is correct. Use `rms evolve-contract module.yaml --task "<task>"` when public compatibility requires deeper guidance, and `rms evidence module.yaml --task "<task>"` when proof design is unclear.

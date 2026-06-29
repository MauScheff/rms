# RMS Optional Runtime Plan

This is a future-facing plan for an optional RMS runtime layer.

The current RMS value proposition remains CLI-first: agents use RMS to shape reliable software, generate semantic structure, verify contracts, and inspect local trace evidence. No module should require an RMS runtime to be valid, testable, replayable, or diagnosable.

## Boundary

The runtime is optional infrastructure for systems that want live message routing, journaling, replay, subscriptions, effect dispatch, and diagnostics across module boundaries.

It must not become the source of RMS semantics. Manifests, contracts, implementation bindings, and evidence remain canonical.

## Goals

- Route commands, events, effects, and effect results through explicit envelopes.
- Persist append-only transition records and effect lifecycle records.
- Reconstruct timelines from journals.
- Replay recorded inputs against declared machines when an implementation opts into executable replay.
- Diagnose the first bad transition from recorded state, input, output, and source provenance.
- Keep projections passive: projections observe records and derive facts or timelines; they do not command workflows.
- Keep workflows explicit: orchestration, subscriptions, deadlines, compensation, and retries are modeled as declared workflow behavior rather than hidden listener chains.

## Non-Goals

- Do not require RMS applications to link a runtime library.
- Do not require a single process, deployment topology, transport, database, or implementation language.
- Do not infer module semantics from runtime behavior.
- Do not allow runtime configuration to override manifests or public contracts.
- Do not hide effects behind implicit callbacks.

## Interoperability Model

The runtime boundary is semantic, not technological.

Participants exchange serialized RMS messages:

- command envelopes ask a target machine to do something;
- event envelopes report facts from a source machine;
- effect envelopes request contact with the outside world;
- effect-result envelopes report the observed result of an effect;
- transition records capture state before, input, output, state after, and source provenance.

Any participant that can produce and consume these semantic records can interoperate. The runtime should treat implementations as black boxes behind declared public entrypoints and contracts.

## Minimum Runtime Roles

- `message_router`: accepts envelopes and delivers them to declared public entrypoints.
- `journal`: persists transition records and effect lifecycle records.
- `replay_reader`: reads journals and builds replay bundles.
- `timeline_projection`: derives passive timelines and facts from journals.
- `subscription_registry`: maps observed events or effect results to workflow inputs.
- `effect_dispatcher`: sends declared effects through adapters and records effect results.
- `diagnostic_projector`: identifies first bad transition candidates and evidence gaps.

## Required Discipline

- All routed messages carry identity, correlation, causation, and schema/version metadata where applicable.
- Effect dispatch is explicit and idempotency-aware.
- Cross-module calls go through public contract-shaped entrypoints.
- Private representation and transition internals stay private.
- Journals record enough information to explain bad states without relying on logs alone.
- Runtime records are evidence; canonical artifacts remain authority.

## CLI-First Bridge

Before a runtime exists, `rms trace check`, `rms trace replay`, and `rms trace diagnose` operate on local trace bundles.

The optional runtime should emit the same kind of bundles, so existing CLI diagnostics continue to work unchanged.

## Phased Path

1. Stabilize the local trace bundle shape used by the CLI.
2. Add optional adapters that write trace bundles from tests, simulations, and local executions.
3. Add an optional journal format with append-only transition and effect lifecycle records.
4. Add optional message routing over declared public entrypoints.
5. Add optional live projections and diagnostic views.
6. Add compatibility checks for runtime record schema evolution.

Each phase must be useful without requiring the next one.

# Law Evidence: structured intent precedes architecture

Promise: architecture-sensitive `next` and `design` work requires a valid `rms/intent-model/v0.1`; material unknowns and forbidden architecture fields stop deterministic topology selection.

Scenarios:

- The exact Client Account Access intent produces one standalone Swift `domain-engine` despite containing negated surface language.
- Missing intent returns `intent.model-required`.
- Unknown material facts return `intent.material-unknown`.
- A model containing `topology` is rejected by the closed schema.
- Provider-only schema drift is deterministically projected into the closed model: descriptive subjects become stable IDs; extra top-level semantic facts and nested fact detail such as `decisions` or `must_never_happen` are discarded; implementation languages become binding preferences; only canonical runnable-surface categories survive; and architecture fields still fail closed.

Command/tool:

- `cargo test -p rms structured_intent_`
- `cargo test -p rms intent_model_requires_material_facts_and_forbids_architecture_fields`
- `cargo test -p rms provider_intent_projection_preserves_closed_schema`

Expected result: all typed-intent validation and deterministic-policy regressions pass without raw-purpose architecture inference.

Source revision: strict audit binds this evidence to the committed candidate.

# Contract Evidence: design-rms-system

Promise: `design-rms-system` validates `rms/intent-model/v0.1` facts before architecture, rejects model fields that contain topology, and applies one language-neutral deterministic topology policy. Provider extraction is read-only evidence, not architecture authority.

Executable scenarios:

- `structured_intent_keeps_pure_reusable_swift_library_standalone` proves the exact Client Account Access intent yields one Swift `domain-engine`, no surface, no composite, and no exception despite negated runnable language.
- `structured_intent_recommends_generic_domain_and_boundary_topology` proves decisions plus a required runnable surface select a recursive decision/boundary topology.
- `structured_intent_names_runtime_monitor_without_keyword_inference` proves responsibility kinds, rather than prose keywords, select specialized shapes.
- `intent_model_requires_material_facts_and_forbids_architecture_fields` proves missing, unknown, contradictory, source-mismatched, and topology-bearing models stop before scaffolding.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked structured_intent_
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked intent_model_requires_material_facts_and_forbids_architecture_fields
```

Expected result: all tests pass and no runtime architecture decision scans raw task or purpose text.

Source provenance: the clean committed candidate revision resolved by `rms audit --root . --strict`.

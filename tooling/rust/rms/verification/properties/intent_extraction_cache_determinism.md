# Property evidence: intent extraction cache determinism

Input space crosses exact task bytes, schema digest, prompt version, provider, model, normalization version, refresh mode, cache validity, and Git revision.

Oracle:

- identical key material produces one repository-local cache key across commits
- task, schema, provider, model, prompt, or normalization changes produce another key
- invalid extractions are not cacheable
- a valid material unknown is cacheable and routes to clarification without repair

Execution:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml intent_extraction_cache_is_deterministic`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml provider_intent_candidate_preserves_material_unknown`

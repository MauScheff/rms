# Law evidence: provider intent extraction is structured, cached, and auditable

Codex extraction uses `--output-schema`, preserves attempt responses and diagnostics, permits one repair for invalid structure, never repairs a valid material unknown, and caches only schema-valid normalized intent under a repository-local key and lock.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml provider_intent_candidate_preserves_material_unknown`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml intent_extraction_cache_is_deterministic`

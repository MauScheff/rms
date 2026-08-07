# Evidence: law proves provider-intent-extraction-is-structured-cached-and-auditable

Promise:

- provider-intent-extraction-is-structured-cached-and-auditable

Scenario:

- Run structured extraction through success, repair, invalid response, incompatibility, quota exhaustion, read-only provider state, default and one-run profiles, refresh, cache hit, and concurrent lookup.
- Force the provider process to reject required structured output before it writes a response.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml provider_pipeline_repairs_caches_refreshes_and_deduplicates --no-fail-fast`

Expected result:

- Operational failure performs no schema-repair attempt and writes the request, prompt, schema, raw attempt, normalized-null intent, diagnostics, provider metadata, terminal route, and non-ready receipt.
- The receipt has no owner, targets, scaffold, or allowed action families.
- Quota exhaustion and read-only provider state have distinct diagnostics and preserve caller-authored fail-closed recovery.
- A one-run profile reaches Codex execution, appears in provider evidence, and changes the cache identity only for that invocation.
- Valid cache behavior and the one-repair ceiling remain unchanged.

Source revision: recorded by git commit or strict audit provenance before production use.

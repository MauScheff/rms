# Property Evidence Obligation: intent-extraction-cache-determinism

This file is an evidence obligation, not observed production proof.

Promise:

- Property `intent-extraction-cache-determinism` proves `provider-intent-extraction-is-structured-cached-and-auditable`.

Input space:

```yaml
attempts: valid, repairable, materially unknown, schema-invalid, provider-process-failed, provider-incompatible, timed-out, cache-hit, refresh, and concurrent provider outcomes
```

Oracle:

- validated identical requests reuse one normalized model without another provider invocation
- invalid structured responses perform at most one repair and failed extraction is never cached
- operational failures perform no schema repair and produce diagnostics, provider metadata, terminal route, and a non-ready receipt with no owner or allowed actions
- every invocation preserves exact raw response and provider logs or cache provenance

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml provider_pipeline_repairs_caches_refreshes_and_deduplicates --no-fail-fast`

Observed result:

- Focused realization passed for valid extraction, one repair, exhausted validation, provider incompatibility, cache hit, refresh, and concurrent deduplication.
- The provider-incompatible case made one attempt, produced all ten required artifacts, selected no owner, and issued an empty-action receipt.
- No counterexample was produced. Any future counterexample belongs under `verification/fuzz/counterexamples/intent-extraction-cache`.

Source revision: recorded by git commit or strict audit provenance before production use.

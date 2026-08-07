# Property Evidence Obligation: intent-extraction-cache-determinism

This file is an evidence obligation, not observed production proof.

Promise:

- Property `intent-extraction-cache-determinism` proves `provider-intent-extraction-is-structured-cached-and-auditable`.

Input space:

```yaml
attempts: valid, repairable, materially unknown, schema-invalid, provider-process-failed, provider-incompatible, quota-exhausted, read-only-state, timed-out, default-profile, one-run-profile, cache-hit, refresh, and concurrent provider outcomes
```

Oracle:

- validated identical requests reuse one normalized model without another provider invocation
- invalid structured responses perform at most one repair and failed extraction is never cached
- operational failures perform no schema repair and produce diagnostics, provider metadata, terminal route, and a non-ready receipt with no owner or allowed actions
- quota exhaustion and read-only provider state have distinct truthful classifications and caller-authored fail-closed recovery artifacts
- a one-run Codex profile is forwarded to provider execution, recorded in evidence, and separated in the cache identity
- every invocation preserves exact raw response and provider logs or cache provenance

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml provider_pipeline_repairs_caches_refreshes_and_deduplicates --no-fail-fast`

Observed result:

- Focused realization passed for valid extraction, one repair, exhausted validation, provider incompatibility, quota exhaustion, read-only provider state, default and one-run profiles, cache hit, refresh, and concurrent deduplication.
- The provider-incompatible case made one attempt, produced all ten required artifacts, selected no owner, and issued an empty-action receipt.
- Quota and read-only-state failures selected no owner, authorized no action, and recorded a schema-valid caller-authored intent template plus an exact recovery command.
- The one-run `cheaper` profile reached `codex exec --profile cheaper` and used a distinct cache key.
- No counterexample was produced. Any future counterexample belongs under `verification/fuzz/counterexamples/intent-extraction-cache`.

Source revision: recorded by git commit or strict audit provenance before production use.

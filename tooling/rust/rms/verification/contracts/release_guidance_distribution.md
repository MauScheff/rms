# Contract Evidence: release guidance distribution

Promise:

- `release-check` rejects drift among canonical guidance assets, embedded RMS skills, plugin skills, root guidance, and managed Codex/Claude copies.
- Release checks preserve unrelated local skills and mechanically enforce onboarding and production completion order.
- The installed release binary smoke includes `rms next` and no release check invokes an optional AI provider.

Deterministic scenarios:

- Synchronize the managed catalog, then independently alter a guidance asset, embedded skill, plugin skill, root guidance block, Codex-local managed copy, and Claude-local managed copy. Each managed mutation must fail the distribution check and identify its path.
- Add unrelated local skills to Codex and Claude targets. The distribution check remains successful and the files retain their bytes.
- Scan README, quickstart, tool documentation, Codex integration guidance, and generated assets. Enforce `init < authorized bootstrap commit < design < recommended scaffold` and `focused checks < gate < authorized candidate commit < strict audit`.
- Reject default capability child-name overrides and require standalone and recursive scaffold examples to be presented as alternatives.
- Install the release binary in an isolated prefix and construct a `next` report for a fixture without provider configuration or invocation.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked release_guidance_distribution_detects_drift_and_allows_unrelated_skills -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked release_documentation_order_is_mechanically_enforced -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked release_binary_smoke_includes_next_command -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked embedded_init_agent_skills_match_canonical_source_when_available -- --nocapture
cargo run --manifest-path tooling/rust/rms/Cargo.toml -- release check --root .
```

Acceptance oracle:

- Any managed-file byte drift fails with a deterministic remediation path; unrelated extra local skills do not affect the result.
- Documentation order checks compare semantic anchors, not incidental line numbers, and reject missing or inverted steps.
- Quickstart offers standalone and recursive scaffolds as mutually exclusive choices after design and contains no invented default child-name flags.
- Installed-binary smoke constructs a deterministic `rms next --task ... --json` report and checks its result/repository fields without invoking a provider or mutating the fixture.
- Release success is possible only when every enabled release obligation passes; commit evidence remains subject to host authorization and final strict audit.

Verification status: this file declares the deterministic proof protocol and does not assert an observed pass. Source provenance and executed results are resolved from the authorized candidate commit by `rms audit --root . --strict`.

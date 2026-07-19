# Contract evidence: configure-agent-integration v2

Managed Codex and Claude guidance begins natural-language work with `rms next "<exact user task>" --root . --ai`, while documenting typed intent inputs for CI and offline callers.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml agent_guidance_assets_are_exact_and_within_budgets`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml release_guidance_distribution_detects_drift_and_allows_unrelated_skills`

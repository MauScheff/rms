# Contract evidence: configure-agent-integration v2

Managed Codex and Claude guidance begins software-change work with `rms next "<exact change task>" --root . --ai`, while reserving read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, and discussion that requests no change for native project tools. It documents typed intent inputs for CI and offline callers.

Verification:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml agent_guidance_assets_are_exact_and_within_budgets`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml release_guidance_distribution_detects_drift_and_allows_unrelated_skills`

# Contract Evidence: repository and skill-source diagnosis

Promise:

- `diagnose-rms-environment` and `recommend-next-rms-work` derive repository kind from one shared `RepositoryProfile` model.
- Diagnosis reports observable RMS skill sources in deterministic order without claiming that the host activated a source in the running agent thread.
- Shape-inapplicable root artifacts are `not-applicable`; malformed or partial applicable roots remain missing, invalid, or blocked evidence.

Deterministic scenarios:

- Build each supported repository shape, including this repository's multi-system root, a direct system root, a direct module root, a container with one system, a workspace with multiple systems, a module-only workspace, and fresh/existing uninitialized directories.
- Assert that diagnosis and next-work planning serialize the same repository kind and root artifact applicability for each fixture.
- Populate project-local Codex and Claude skills, known direct user skill locations, personal marketplace configuration, enabled and disabled plugins, and several plugin-cache versions.
- Exercise byte-identical duplicates, divergent copies, stale cache versions, absent home directories, and unreadable optional locations. Sort origins by stable scope/origin/path keys and calculate SHA-256 from file bytes.
- Verify every origin reports scope, path, configured state, digest, equivalence to the embedded managed skill, `runtime_activation: unknown`, and `precedence: host-defined`.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked diagnose_reports_repository_profile_and_detected_skill_summary -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked skill_sources_distinguish_equivalent_divergent_stale_and_configured_origins -- --nocapture
```

Acceptance oracle:

- Repository-kind and applicability fields are identical wherever diagnose and next inspect the same root.
- Missing root `system.yaml` and `context-map.yaml` are `not-applicable` for container/workspace/module-root shapes, but a root that begins canonical ownership cannot hide missing or invalid artifacts behind `not-applicable`.
- Identical digests are informational; divergent managed copies are `review-required` and prescribe the applicable project-sync or plugin-sync remedy.
- Plugin configured state is derived from observable marketplace/configuration data, not cache presence alone. Cache versions have stable current/stale classification.
- An absent home or optional source location yields an explicit absent/unavailable observation rather than a fabricated active source.
- No output field claims runtime activation or resolves host precedence.

Verification status: this file declares the deterministic proof protocol and does not assert an observed pass. Source provenance and executed results are resolved from the authorized candidate commit by `rms audit --root . --strict`.

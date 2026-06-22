# Contract Evidence: release-check

Covered by `cargo test --manifest-path Cargo.toml` and by release-check smoke execution.

Executable coverage:

- `release_metadata_detects_version_drift` verifies release readiness rejects version drift across the Cargo package, `rms-cli` module manifest, and packaged Codex plugin manifest.
- `codex_plugin_sync_detects_packaged_skill_drift` verifies the portable Codex plugin check accepts synced skills and rejects packaged skill drift.
- The release-check smoke path runs `rms release check --skip-cargo-package`, which executes release metadata checks, formatting, Rust tests, RMS validation, RMS implementation verification, composition and compatibility smokes, package creation and verification smoke, example binding tests, release-binary smoke, and Codex plugin sync validation without invoking optional AI providers.
- The release-binary smoke builds `target/release/rms`, runs that binary directly for `rms diagnose --root . --json` and `rms validate --root examples/minimal`, then copies it into a temporary install directory, prepends that directory to `PATH`, and runs `rms` by name for the same deterministic smoke path.
- Full release readiness includes `cargo package --manifest-path tooling/rust/rms/Cargo.toml --allow-dirty --no-verify`; `--skip-cargo-package` is reserved for offline checks.

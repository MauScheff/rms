# Law Evidence: reusable package proof is current

Promise:

- Law `reusable-package-proof-is-current` requires a reusable module to rebuild and independently verify from current source; an existing payload must match the regenerated manifest.

Scenario:

- Build and verify a reusable RMS package.
- Corrupt an existing package payload checksum, then regenerate the package in a temporary directory.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml package_command_records_and_packages_reuse_proof`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml strict_package_regeneration_rejects_existing_stale_payload`

Expected result:

- Current packages pass independent `verify-package` checks.
- Existing stale output reports `package.proof-stale` even when the temporary rebuild succeeds.

Observed result: both focused regressions passed. Source revision is resolved from the committed candidate by strict audit.

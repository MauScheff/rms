# Contract Evidence: package-module

Promise:

- `package-module` assembles a closed RMS package, verifies its checksums and manifests, records concrete proof for reusable modules, rebuilds, and verifies the final artifact before returning success.

Scenario:

- Package a reusable fixture whose declared evidence initially contains only the obligation and expected result.
- Confirm RMS writes a marked recorded-result section and that the rebuilt package contains the same proof.
- Verify the final package independently and tamper with a payload to exercise explicit verification failure.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml package_command_records_and_packages_reuse_proof`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml verify_package_accepts_clean_package_and_rejects_tampering`

Expected result:

- Successful packaging prints both assembly and verification results.
- Reusable evidence and the final package payload contain the recorded pass.
- Missing, unsafe, invalid, or tampered artifacts produce explicit command failure.

Source revision: resolved from the candidate Git commit by strict audit.

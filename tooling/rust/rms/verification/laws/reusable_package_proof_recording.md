# Law Evidence: reusable package proof recording

Promise:

- `reusable-package-proof-is-cli-recorded`: package evidence distinguishes an expected result from a successful package and checksum verification actually observed by RMS.

Scenario:

- A reusable module begins with a concrete package obligation but no recorded pass.
- `rms package` assembles and verifies the artifact, writes a marked recorded-result block to the declared reuse evidence, rebuilds the package with that proof, and verifies the final artifact.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml package_command_records_and_packages_reuse_proof`

Expected result:

- Both source evidence and the final package payload contain the recorded package result.
- `rms verify-package` passes for the rebuilt artifact and `semantic.reusable-package-evidence-missing` no longer accepts expectation-only prose.

Source revision: resolved from the candidate Git commit by strict audit.

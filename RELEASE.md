# Release Process

This repository releases the RMS specification, canonical skills, Codex adapter, examples, and the `rms` reference CLI as one coherent artifact set.

The release authority is the repository state at a signed or reviewed tag. Generated binaries, source crates, checksums, packaged skills, and documentation are distribution artifacts. They must not redefine RMS semantics.

## Release Surface

| Artifact | Path or command | Purpose |
|---|---|---|
| RMS CLI crate | `tooling/rust/rms/Cargo.toml` | Source package for the `rms` binary. |
| CLI module bundle | `tooling/rust/rms/module.yaml` | Canonical module contract for the workbench itself. |
| Release contract | `tooling/rust/rms/contracts/release-check.v1.yaml` | Deterministic release-readiness promise. |
| Codex plugin wrapper | `integrations/codex/rms/` | Installable adapter that packages canonical skills. |
| Canonical skills | `skills/*/SKILL.md` | Source of agent workflows. |
| Release workflow | `.github/workflows/release.yml` | Tag-driven source and binary artifact publication. |

## Version Rules

These versions must match before a release is accepted:

```text
tooling/rust/rms/Cargo.toml              package.version
tooling/rust/rms/module.yaml             module.version
integrations/codex/rms/.codex-plugin/plugin.json  version
```

The tag must be `v<version>`, for example `v0.1.0` or `v0.1.0-rc.1`. The release workflow checks that the tag version matches the Cargo package version. `rms release check` checks the Cargo, RMS module, and Codex plugin versions.

## Local Release Gate

Run the canonical gate from a clean working tree:

```bash
cargo run -p rms --locked -- release check --root .
git diff --check
```

The gate runs release metadata checks, formatting, Rust tests, RMS validation, RMS implementation verification, example composition, compatibility smoke tests, RMS package creation and verification smoke tests, scaffold roundtrips, example binding tests, release-binary smoke, clean-room PATH install smoke, Cargo packaging, and Codex plugin sync validation. It does not invoke optional AI providers.

Use `--skip-cargo-package` only for offline local checks where crates.io index access is unavailable.

## Tag Release

1. Update versions in the Cargo package, `rms-cli` module manifest, and Codex plugin manifest.
2. Update `CHANGELOG.md`.
3. Run the local release gate.
4. Create and push the tag:

```bash
git tag v<version>
git push origin v<version>
```

Pushing a `v*` tag runs `.github/workflows/release.yml`. The workflow:

1. verifies the tag version;
2. runs `rms release check --root .`;
3. builds runner-native release binaries for Linux, macOS, and Windows;
4. packages the Rust source crate;
5. emits SHA-256 checksum files;
6. attaches artifacts to the GitHub release.

## Release Candidate

Use a release-candidate version only when the artifact should be exercised before the final tag:

```text
0.1.0-rc.1
v0.1.0-rc.1
```

The workflow requires the tag version to match the Cargo package version exactly after removing the leading `v`. If an RC is cut, update the Cargo package, `rms-cli` module manifest, and Codex plugin manifest to the same RC version before tagging.

## Expected Artifacts

Each GitHub release should contain:

```text
rms-<tag>-Linux-<arch>.tar.gz
rms-<tag>-Linux-<arch>.tar.gz.sha256
rms-<tag>-macOS-<arch>.tar.gz
rms-<tag>-macOS-<arch>.tar.gz.sha256
rms-<tag>-Windows-<arch>.zip
rms-<tag>-Windows-<arch>.zip.sha256
rms-<version>.crate
rms-<tag>-crate.sha256
```

Runner architecture is determined by GitHub Actions. The artifacts are native to the runner that built them.

## Install Paths

For release users, prefer the GitHub release archive for their platform, then place `rms` on `PATH`.

For source users:

```bash
cargo install --path tooling/rust/rms
```

For contributors who do not want to install:

```bash
cargo run -p rms -- validate --root examples/minimal
```

After installation:

```bash
rms diagnose
```

Inside a source checkout:

```bash
rms explain "How does this module work?" --root examples/minimal
rms release check --root .
```

## Done Criteria

A release is complete when:

- the release workflow is green;
- every expected artifact and checksum is attached;
- `rms diagnose` works from an extracted binary archive;
- release-binary and clean-room PATH install smoke pass in `rms release check --root .`;
- `rms release check --root .` passes from source;
- packaged Codex skills match canonical `skills/`;
- the release notes identify compatibility impact and operational caveats.

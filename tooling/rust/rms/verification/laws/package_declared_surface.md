# Law Evidence: Packages Carry Declared Implementation Surface

Promise:

- Invariant `rms-package-carries-declared-implementation-surface` holds.
- `rms package <module.yaml>` includes the sibling `implementation.yaml`, declared role files, the public facade, scripts referenced by build/verify commands, conformance report, source revision, and checksums.
- `rms verify-package <package-dir>` rejects tampered payloads.

Command/tool:

- `cargo test -p rms package -- --nocapture`
- `rms package tooling/rust/rms/module.yaml --output <package-dir> --force`
- `rms verify-package <package-dir>`

Expected result:

- `package_includes_manifest_references_and_metadata` asserts that packaged fixtures include `implementation.yaml`, `src/public.mjs`, `src/transition.mjs`, `src/representation.mjs`, referenced scripts, verification evidence, conformance report, and `PACKAGE.json`.
- `verify_package_accepts_clean_package_and_rejects_tampering` accepts the generated package and then fails with `package.file.sha256` after a payload file is modified.
- Package metadata remains RMS-owned; native package files are optional binding evidence and do not define reusable semantics.

Source revision: recorded by the git commit that includes this evidence and enforced by strict audit provenance.

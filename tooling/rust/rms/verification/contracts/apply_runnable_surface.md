# Contract Evidence: apply-runnable-surface

Promise: `apply-runnable-surface` records runnable boundary surfaces in canonical implementation artifacts before source code owns that architecture.

Command/tool:

- `cargo test --manifest-path Cargo.toml surface_apply_records_surface_role_and_evidence`

Expected result:

- `rms surface apply` records `architecture.surfaces`.
- `architecture.roles.runnable_surface` includes the entrypoint.
- Concrete boundary evidence is created for the delegation promise.

Source revision: recorded by git commit or strict audit provenance before production use.

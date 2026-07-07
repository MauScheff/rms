# Contract Evidence: apply-runnable-surface

Promise: `apply-runnable-surface` records runnable boundary surfaces in canonical implementation artifacts before source code owns that architecture.

Command/tool:

- `cargo test --manifest-path Cargo.toml surface_apply_records_surface_role_and_evidence`
- `cargo test --manifest-path Cargo.toml surface_apply_browser_generates_thin_controller_launch_assets`

Expected result:

- `rms surface apply` records `architecture.surfaces`.
- `architecture.roles.runnable_surface` includes the entrypoint.
- Concrete boundary evidence is created for the delegation promise.
- Browser surfaces generate missing thin launch assets: controller entrypoint, launch script, and host HTML.
- The launch script imports the declared controller, and the controller delegates to the declared boundary adapter/public entrypoint.

Source revision: recorded by git commit or strict audit provenance before production use.

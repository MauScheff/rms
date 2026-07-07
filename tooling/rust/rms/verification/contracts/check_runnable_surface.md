# Contract Evidence: check-runnable-surface

Promise: `check-runnable-surface` catches runnable app, UI, CLI, browser, HTTP, batch, or executable surfaces that bypass declared RMS boundary roles.

Command/tool:

- `cargo test --manifest-path Cargo.toml structure_report_flags_unwired_runnable_surface_with_copied_logic`
- `cargo test --manifest-path Cargo.toml structure_report_flags_runnable_surface_bypassing_boundary_adapter`
- `cargo test --manifest-path Cargo.toml add_capability_browser_tool_declares_thin_runnable_surface`
- `cargo test --manifest-path Cargo.toml surface_apply_browser_generates_thin_controller_launch_assets`
- `cargo test --manifest-path Cargo.toml surface_check_flags_launch_script_with_copied_domain_logic`

Expected result:

- Disconnected runnable surfaces report `structure.runnable-surface-not-wired`.
- Copied decision logic in runnable surfaces reports `structure.runnable-surface-domain-logic-duplication`.
- Generated browser surfaces route host HTML through launch script, declared controller, and boundary adapter.
- Launch scripts that duplicate domain decisions still fail runnable-surface diagnostics.

Source revision: recorded by git commit or strict audit provenance before production use.

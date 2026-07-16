# Law Evidence: simple runnable boundaries are stateless by default

Promise:

- Simple runnable app, tool, browser, and CLI boundary scaffolds use a stateless boundary machine unless the product intent names real lifecycle/order/session/retry/status/recovery/workflow behavior.
- Browser scaffolds distinguish the declared controller entrypoint from the host launch file and launch script.
- Non-effectful JS scaffolds do not declare fake effect lifecycle or effect-result semantics.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml add_capability_browser_tool_declares_thin_runnable_surface`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml surface_apply_browser_generates_thin_controller_launch_assets`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml js_boundary_adapter_scaffold_separates_representation_parser_and_adapters`

Expected result:

- `rms add-capability-tree` for local browser tool intent creates a boundary implementation whose machine mode is `stateless-decision-machine` and whose only starter state is `AwaitingInput`.
- Generated browser assets are `public/index.html`, `public/app.mjs`, and declared controller `public/controller.mjs`.
- The launch script imports the declared controller, and the controller imports the boundary adapter instead of duplicating domain decisions.
- Simple JS boundary scaffolds omit `effect_lifecycle` and generated `EffectResultEnvelope` semantics unless effects are actually declared.

Source revision: recorded by git commit before production release.

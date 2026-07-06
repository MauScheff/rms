# Runnable Launch Assets Route Through Boundaries

Promise:

`runnable-launch-assets-route-through-boundaries`

Launch assets reachable from a runnable surface route through the declared RMS
entrypoint or adapter. They must not duplicate parser, generator, transition, or
domain decision logic outside declared roles.

Scenario:

- A boundary module declares a browser runnable surface with controller
  `public/app.mjs` and launch file `public/index.html`.
- The launch file also loads `public/app.browser.js`.
- `public/app.browser.js` contains copied parser/generator-style decision
  functions and does not call the declared boundary adapter.
- RMS inspects scripts discovered from the launch file, not only the declared
  controller, and reports the launch asset as disconnected duplicated surface
  logic.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml surface_check_flags_launch_script_with_copied_domain_logic`
- `cargo test --workspace --locked`
- `target/debug/rms surface check modules/tile-browser/implementation.yaml --strict`
  in the blind Tile Generator dogfood project

Expected result:

- The unit test produces `structure.runnable-surface-domain-logic-duplication`
  and `structure.runnable-surface-bypasses-boundary` for
  `public/app.browser.js`.
- The same strict surface check fails the blind Tile Generator dogfood project
  because the actual browser-loaded script bypasses the declared RMS boundary.
- Generated and project-local agent guidance instructs agents to declare browser
  launch files and local scripts with RMS surface semantics instead of treating a
  secondary bundle as invisible implementation detail.

Source revision: recorded by the git commit that lands this evidence with the
diagnostic implementation.

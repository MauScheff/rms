# Scenario Evidence: Reusable Module Package

Promise:

- `rms add-capability-tree` scaffolds reusable pure/domain children as capability providers with package/reuse evidence.
- JS reusable/domain bindings expose `src/public.mjs` as the public facade while keeping `representation.mjs` and `transition.mjs` as role files.
- Consumers compose through `requires.capabilities[]` and contract paths, not private source imports.

Command/tool:

- `cargo test -p rms add_capability -- --nocapture`
- `cargo test -p rms package -- --nocapture`
- `cargo test -p rms native_package -- --nocapture`
- `rms compose --root .`
- `rms gate --root .`

Expected result:

- `add_capability_scaffolds_recursive_tree_that_verifies` confirms the generated domain child publishes the reusable capability, records `x-rms.reusable: true`, and includes `verification/scenarios/reusable_package.md`.
- `add_capability_browser_tool_declares_thin_runnable_surface` confirms JS domain scaffolds write `distribution.public_facade: src/public.mjs` and create the facade file.
- Composition keeps the boundary-to-domain dependency contract explicit.
- Native package export mismatches and cross-module private role imports remain strict-audit blockers.

Source revision: recorded by the git commit that includes this evidence and enforced by strict audit provenance.

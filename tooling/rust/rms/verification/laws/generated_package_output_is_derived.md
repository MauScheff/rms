# Evidence: law proves generated-package-output-is-derived

Promise:

- generated-package-output-is-derived

Scenario:

- Place a packaged copy of an RMS module under `dist/` beside the source module, then discover and validate the project.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml generated_package_output_is_not_rediscovered_as_live_project_semantics`

Expected result:

- RMS discovers the source module exactly once and ignores canonical-looking artifacts under generated package and build trees.

Source revision: recorded by git commit or strict audit provenance before production use.

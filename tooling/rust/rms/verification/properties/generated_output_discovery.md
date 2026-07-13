# Property Evidence: generated-output-discovery-is-excluded

Promise:

- Property `generated-output-discovery-is-excluded` proves `generated-package-output-is-derived`.

Input space:

```yaml
fixtures:
- source module beside dist package copy
- target build tree
- node_modules dependency tree
```

Oracle:

- only source-owned canonical artifacts are discovered
- derived package copies do not create duplicate modules

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml generated_package_output_is_not_rediscovered_as_live_project_semantics`

Expected result:

- The fixture passes only when discovery returns the source module and excludes its packaged copy under `dist/`.

Source revision: recorded by git commit or strict audit provenance before production use.

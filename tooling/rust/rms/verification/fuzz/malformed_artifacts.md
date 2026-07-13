# Fuzz Evidence: rms-cli-malformed-artifacts-produce-diagnostics

Promise:

- Fuzz target `rms-cli-malformed-artifacts-produce-diagnostics` proves `canonical-artifacts-remain-authoritative`.

Input space:

```yaml
cli_args: generated malformed or incomplete command arguments
manifests: generated malformed RMS YAML and schema-violating artifacts
```

Oracle:

- malformed input produces diagnostics or explicit command errors
- rejected input does not create semantic authority outside RMS canonical apply paths
- diagnostics remain derived evidence rather than hidden architecture

Command/tool:

- `rms property check tooling/rust/rms/module.yaml --strict`
- `rms property run tooling/rust/rms/implementation.yaml --profile smoke`
- Binding command: `cargo test --manifest-path Cargo.toml schema_validation_reports_shape_errors` from `tooling/rust/rms`.
- Declared harness: `src/main.rs#generate_malformed_artifact_cases`.

Expected result:

- Schema-violating RMS artifacts are rejected with diagnostics.
- The named harness generated three distinct malformed module shapes and the focused test passed all three cases.
- Rejected artifacts do not become module, implementation, contract, or evidence authority.
- Future generated/adversarial failures should be recorded under `verification/fuzz/counterexamples` with `spec: rms/property-counterexample/v0.1`.

Source revision: recorded by git commit or strict audit provenance before production use.

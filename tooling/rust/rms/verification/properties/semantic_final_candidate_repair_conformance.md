# Property Evidence: semantic final-candidate repair conformance

Promise:

- Property `semantic-final-candidate-repair-conformance` proves `semantic-apply-validates-final-candidate`.

Input space:

```yaml
current_manifest: a schema-invalid module manifest
corrective_change: a semantic change whose final artifacts and transformations are valid
semantic_ids: stable identifiers containing hyphens or underscores
```

Oracle:

- semantic apply and JSON Schema accept the same stable language-neutral identifiers
- invalid current state does not block a valid computed final candidate
- dry-run and write mode validate the same final candidate

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml spec_apply_repairs_schema_invalid_current_artifact_state`

Observed result:

- Passed one deterministic regression containing a schema-invalid current module, a corrective dry run, a corrective write, hyphenated artifact and transformation IDs, and final embedded-schema validation.
- Dry-run left the invalid source byte-for-byte unchanged; write mode produced the same valid semantic model.
- No counterexample was produced.

Source revision: recorded by git commit or strict audit provenance before production use.

# Property Evidence: semantic properties

Promise:

- `semantic-properties-are-canonical-reliability-evidence`
- `semantic-property-declarations-have-oracles`

Input space:

```yaml
semantic_changes:
  - rms/semantic-change/v0.1 records with properties.add
implementation_bindings:
  - implementation.yaml files with architecture.reliability.properties
  - implementation.yaml files with architecture.reliability.fuzz_targets
counterexamples:
  - rms/property-counterexample/v0.1 files
```

Oracle:

- every semantic property names what it proves;
- every semantic property declares an input space;
- every semantic property declares at least one oracle;
- property and fuzz targets reference concrete evidence;
- binding commands are declared under `commands.properties` or `commands.fuzz` before `rms property run`;
- counterexamples carry replay metadata through a replay command or trace reference.

Command/tool:

- `cargo test --workspace --locked property`
- `cargo run --manifest-path tooling/rust/rms/Cargo.toml -- property check tooling/rust/rms/module.yaml`
- `cargo run --manifest-path tooling/rust/rms/Cargo.toml -- property check tooling/rust/rms/module.yaml --strict`

Expected result:

- property and fuzz verification lanes are recognized as canonical evidence categories;
- `rms property check` reports complete semantic property declarations for `rms-cli`;
- generated bindings may implement the property in any language-native way, but the semantic input space and oracle remain in RMS artifacts.

Source revision: recorded by git commit and strict audit provenance before production use.

# Contract Evidence: run-semantic-properties

Promise:

- `run-semantic-properties` exposes `rms property run` for implementation bindings that declare `commands.properties` or `commands.fuzz`.
- RMS delegates execution to declared binding commands instead of choosing a language-specific property library.

Command/tool:

- `cargo test --workspace --locked property`
- fixture tests exercise generated `commands.properties` and `commands.fuzz` declarations for Rust, Swift, and JS scaffolds.

Expected result:

- missing property/fuzz commands fail with `property.command-missing`;
- declared commands run from the implementation directory;
- the report records command kind, command text, status, and exit code.

Source revision: recorded by git commit and strict audit provenance before production use.

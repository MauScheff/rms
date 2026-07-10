# Contract Evidence: run-semantic-properties

Promise:

- `run-semantic-properties` exposes `rms property run` for implementation bindings that declare semantic property realizations.
- RMS selects the requested realization profile and delegates execution to its declared binding command instead of choosing a language-specific property library.

Command/tool:

- `cargo test --workspace --locked property`
- fixture tests exercise deterministic corpus, deterministic exhaustive, generated-property, and coverage-fuzzer realization metadata independently of binding language.

Expected result:

- missing property/fuzz commands fail with `property.command-missing`;
- selected realization commands run from the implementation directory;
- fixed corpora cannot satisfy open-ended fuzz claims;
- the report records property, profile, strategy, command text, status, and exit code.

Source provenance: the candidate commit and this command are recorded by strict audit before a production claim.

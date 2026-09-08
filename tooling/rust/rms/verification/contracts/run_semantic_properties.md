# Contract Evidence: run-semantic-properties

Promise:

- `run-semantic-properties` exposes `rms property run` for implementation bindings that declare semantic property realizations.
- RMS selects the requested realization profile and delegates execution to its declared binding command instead of choosing a language-specific property library.

Command/tool:

- `cargo test --workspace --locked property`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml property_run_executes_each_shared_command_realization --no-fail-fast`
- fixture tests exercise deterministic corpus, deterministic exhaustive, generated-property, and coverage-fuzzer realization metadata independently of binding language.

Expected result:

- properties that declare only another profile are outside the selected run and do not fail it;
- a selected profile with no property or fuzz realization fails with `property.command-missing`;
- selected realization commands run from the implementation directory;
- fixed corpora cannot satisfy open-ended fuzz claims;
- the report records property, profile, strategy, command text, status, and exit code;
- replayable files emitted into a declared counterexample directory are discovered and run as durable regressions on the current and every later property run;
- a malformed or still-failing recorded counterexample fails the property run rather than remaining passive evidence.

Source provenance: the candidate commit and this command are recorded by strict audit before a production claim.

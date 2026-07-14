# Property Evidence: runnable usage and smoke resolution

Property `runnable-usage-and-smoke-resolution` covers surface application, generated usage, command resolution, and smoke execution.

Command/tool: `cargo test --manifest-path Cargo.toml verify_executes_declared_runnable_smoke_command`

Observed result: 1 end-to-end deterministic fixture passed. Native verification and traces ran first, then the distinct surface smoke command ran and created its marker. Missing usage and command declarations are covered by declaration diagnostics. No counterexample was produced.

Source revision: resolved from the committed candidate by strict audit.

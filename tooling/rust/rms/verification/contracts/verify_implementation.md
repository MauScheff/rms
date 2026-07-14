# Contract Evidence: verify implementation

Promise: implementation verification runs native tests, declared trace replay, and every distinct runnable smoke command.

Command/tool:

- `cargo test --manifest-path Cargo.toml verify_executes_declared_runnable_smoke_command`
- `cargo test --manifest-path Cargo.toml verify_fails_bad_declared_trace_bundle`

Expected and observed result: valid native, trace, and surface smoke checks pass in order; failed verification or trace evidence remains a command failure. Focused fixtures pass.

Source revision: resolved from the committed candidate by strict audit.

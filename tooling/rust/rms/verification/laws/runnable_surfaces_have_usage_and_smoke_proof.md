# Law Evidence: runnable usage and smoke proof

Promise: invariant `runnable-surfaces-have-usage-and-smoke-proof` requires every runnable surface to resolve a usage document and executed smoke command.

Command/tool: `cargo test --manifest-path Cargo.toml verify_executes_declared_runnable_smoke_command`

Expected and observed result: surface apply creates `USAGE.md`, records the command key, and verification creates the smoke marker only by executing the declared product smoke command. The focused fixture passes.

Source revision: resolved from the committed candidate by strict audit.

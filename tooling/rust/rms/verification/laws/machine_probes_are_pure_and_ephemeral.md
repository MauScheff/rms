# Machine Probes Are Pure and Ephemeral

Promise: a probe executes only declared transition-record functions. It may
schedule canonical messages and substitute declared effect results, but it
never invokes a machine driver, real effect executor, or private module input.

The probe validator resolves exact binding symbols and temporary-file
protocols for Rust, Swift, JavaScript, and Python adapters. Assembly routing
comes only from canonical protocol bindings and dependency probe bridges.
Bounded exploration is deterministic, and a pass means the reachable bounded
space was exhausted. Failures retain their exact check while being minimized
into replayable schedules and fault decisions.

Probe reports remain diagnostics unless the caller explicitly writes them.
Canonical probe evidence is stronger: referenced assemblies must rerun and
exhaust successfully, while referenced counterexamples must replay as
resolved.

Command/tool: `cargo test --workspace --locked probe_`.

Expected result: pure adapters and canonically wired assemblies pass; drivers,
effect executors, invented routes, inconclusive bounds, reproduced failures,
and invalid saved counterexamples fail.

Source provenance: the clean committed candidate revision resolved by
`rms audit --root . --strict`.

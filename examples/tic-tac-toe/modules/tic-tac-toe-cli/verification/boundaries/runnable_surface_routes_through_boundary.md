# Boundary Evidence: runnable surface routes through boundary

Promise:

- Runnable surface `tic-tac-toe-cli-cli` enters module `tic-tac-toe-cli` through declared RMS command `play-tic-tac-toe`.
- Entrypoint `src/adapter.mjs` delegates to `src/adapter.mjs#handleBoundaryInput` before pure decisions run.
- Product behavior is not reimplemented only in the runnable surface.

Command/tool:

- `rms surface check implementation.yaml --strict`
- `rms structure implementation.yaml`
- `rms verify implementation.yaml`

Expected result:

- Surface wiring references the declared boundary adapter, parser, or public entrypoint.
- Malformed boundary input is parsed/rejected before domain delegation.
- Declared boundary effects remain behind adapter, port, or effect-executor roles.

Source revision: recorded by git commit or strict audit provenance before production use.

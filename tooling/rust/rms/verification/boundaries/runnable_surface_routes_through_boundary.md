# Boundary Evidence: runnable surface routes through boundary

Promise:

- Runnable surface `rms-cli-cli` enters module `rms-cli` through declared RMS command `validate-rms-artifacts`.
- Entrypoint `src/main.rs` delegates to `src/main.rs#main` before pure decisions run.
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

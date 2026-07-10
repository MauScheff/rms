# Law Evidence: stateful machine inputs

Promise: `stateful-machines-consume-state-and-classified-input`.

Scenarios:

- Stateful machines without `state-and-input` or an input ADT fail structure checks.
- Transition inputs outside commands, observed events, and effect results fail classification.
- Rust, Swift, and JS scaffolds expose one canonical state-plus-input transition.

Command/tool:

- `cargo test --workspace --locked stateful_transition`
- `rms machine check tooling/rust/rms/implementation.yaml --strict`

Expected result: missing state/input signatures and unclassified inputs fail deterministically; the RMS workbench machine passes.

Source revision: supplied by the enclosing committed RMS candidate and checked by strict audit.

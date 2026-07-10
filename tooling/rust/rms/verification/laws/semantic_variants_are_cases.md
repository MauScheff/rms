# Law Evidence: semantic variants are cases

Promise: `semantic-variants-are-distinct-from-binding-types`.

Scenarios:

- A machine list containing its Rust, Swift, or JS container type is rejected.
- Generated Rust, Swift, and JS manifests keep type names under `architecture.machine.types` and alternatives under the semantic lists.

Command/tool:

- `cargo test --workspace --locked semantic_variants`
- `rms machine check tooling/rust/rms/implementation.yaml --strict`

Expected result: collapsed type declarations fail with `structure.semantic-variants-collapsed-to-type`; RMS's own explicit variants pass.

Source revision: supplied by the enclosing committed RMS candidate and checked by strict audit.

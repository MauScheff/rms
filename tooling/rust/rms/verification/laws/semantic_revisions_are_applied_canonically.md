# Semantic Revisions Are Applied Canonically

Promise:

`semantic-revisions-are-applied-canonically`

Semantic and machine revisions that correct scaffold drift must use canonical
`set`, `remove`, `supersedes`, launch entrypoint, and active-record rules instead
of hand-edited manifest surgery.

Scenario:

- `rms machine apply` accepts `set`, `remove`, and `add` for variants,
  transitions, and roles, computes a final machine state, rejects stale
  references, and reports the final intended state in dry-run output.
- A focused machine change can canonically add the required probe command,
  initial state, probe binding, and probe-adapter role to an explicitly routed
  legacy implementation; dry-run writes nothing and normal validation passes
  after apply.
- `rms spec apply` records semantic-change files, treats superseded change
  records as historical during strict audit, and reuses the same machine
  validation path for embedded machine changes.
- `rms surface apply` supports a runnable controller `entrypoint` plus an
  optional `launch_entrypoint`, and strict surface checks verify browser launch
  files reference the declared controller.
- Generated runnable browser/tool boundaries stay thin while still using a
  canonical state-plus-input transition; product lifecycle is not invented in
  the surface or hidden in its effect executor.

Command/tool:

- `cargo fmt --all --check`
- `cargo test --workspace --locked`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml legacy_binding_migration_routes_plans_applies_and_validates_end_to_end`
- `target/debug/rms validate --root .`
- `target/debug/rms compose --root .`
- `target/debug/rms gate --root .`
- `target/debug/rms release check --root . --skip-cargo-package`

Expected result:

- The unit tests cover semantic/machine revision application, superseded
  semantic-change audit behavior, launch-entrypoint browser scaffolds,
  `NoReplyDeclared` trace rejection, and pure `UnknownCommand` rejection without
  reconciliation false positives.
- Repository validation, composition, gate, and release readiness pass.
- `rms audit --root . --strict` may still fail before commit because strict
  production claims require dirty source and semantic artifacts to be committed
  together.

Source revision: `git:2bd3aaad468a` before this working-tree change; final
production provenance is established by committing this semantic record with the
implementation changes.

# Law Evidence: truthful fuzz realization

Promise: `fuzz-realization-matches-semantic-claim`.

Scenarios:

- A fixed deterministic corpus cannot satisfy an open-ended fuzz target.
- Generated-property and coverage-fuzzer realizations satisfy open input spaces.
- Deterministic exhaustive realization remains valid for a declared finite input space.

Command/tool:

- `cargo test --workspace --locked fuzz_realization`
- `rms property check tooling/rust/rms/implementation.yaml`

Expected result: mismatched claims fail with `evidence.fuzz-realization-mismatch`; RMS's generated-property targets pass.

Source revision: supplied by the enclosing committed RMS candidate and checked by strict audit.

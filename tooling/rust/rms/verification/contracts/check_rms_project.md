# Contract Evidence: check RMS project

Promise:

- `check-rms-project` delegates one explicit readiness mode to existing RMS authorities and projects their result without recursively invoking the RMS executable or duplicating policy.
- Default mode selects validation and composition; environment selects diagnosis; changes selects gate; committed selects strict audit.

Executable scenarios:

- Exercise all four modes with passing and failing fixtures and assert that only the selected shared implementations run.
- Reject mutually exclusive mode flags and unreadable roots.
- Compare compact text, `rms.surface/v2` JSON, and detailed projections, including stable component ordering and delegated diagnostics.
- Assert a mode succeeds only when every selected component passes and that neither projection nor delegation grants Git authority or converts uncommitted state into committed evidence.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked check_facade_delegates_modes_and_preserves_authority -- --nocapture
```

Acceptance oracle:

- Each mode calls the existing diagnosis, validation, composition, gate, or strict-audit implementation directly and returns the aggregate delegated status.
- JSON has `schema: rms.surface/v2`, the shared surface fields, selected mode, and component summaries; `--details` nests complete delegated evidence under that envelope.
- Constructed failing reports exit unsuccessfully, while argument errors retain the CLI parser's argument-error exit behavior.

Verification status: this file declares the executable proof protocol and does not assert an observed pass. Source provenance and executed results are resolved from the authorized candidate commit by `rms audit --root . --strict`.

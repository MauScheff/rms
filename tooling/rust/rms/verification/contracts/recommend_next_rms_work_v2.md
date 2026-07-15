# Contract Evidence: recommend next RMS work v2

Promise:

- `recommend-next-rms-work` constructs a deterministic prospective prescription, or a truthful `no-rms-change` finding, for a nonblank intent without using unrelated Git changes as task evidence.
- Compact text and `rms.surface/v2` JSON derive from the same report; detailed evidence is opt-in and candidate commits remain non-executable host-authorized actions.

Executable scenarios:

- Preserve coverage for every repository kind, explicit/direct/sole/matched/recursive owner resolution, ties, and every established task lane.
- Classify installation, managed skill or plugin synchronization, and Git status/fetch/commit/rebase/merge/push as `repository-operation` with `no-rms-change`; assert semantic intent about RMS behavior takes precedence.
- Exercise initialized, readable uninitialized, invalid canonical, blank-task, and unreadable-input cases. A `no-rms-change` report contains no design, spec, source-edit, gate, audit, or pending-candidate step.
- Compare clean and unrelated-dirty fixtures, compact and detailed projections, safe argument rendering, manual authorization steps, filesystem snapshots, and provider/prescribed-work sentinels.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked next_surface_projects_v2_and_repository_operations -- --nocapture
```

Acceptance oracle:

- Every constructed report uses a declared result and lane, exits successfully, and preserves deterministic owner routing without guessing through ties.
- JSON has `schema: rms.surface/v2`, the shared surface fields, lane, confidence, owner state, and ordered typed steps. Command steps carry `program` plus `args`; manual commit steps carry `host-required` authorization and no Git program.
- Unrelated working-tree state cannot change classification or ownership, and construction performs no writes, verification, provider execution, or prescribed-work execution. Read-only Git revision inspection may run solely to report provenance.

Verification status: this file declares the executable proof protocol and does not assert an observed pass. Source provenance and executed results are resolved from the authorized candidate commit by `rms audit --root . --strict`.

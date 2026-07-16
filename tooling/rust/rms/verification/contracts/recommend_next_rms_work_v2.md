# Contract Evidence: recommend next RMS work v2

Promise:

- `recommend-next-rms-work` constructs a deterministic prospective prescription, clarification, model requirement, or truthful `no-rms-change` finding from a validated typed intent without using unrelated Git changes or raw task keywords as architecture evidence.
- Compact text and `rms.surface/v2` JSON derive from the same report; detailed evidence is opt-in and candidate commits remain non-executable host-authorized actions.

Executable scenarios:

- Preserve coverage for every repository kind, structured-subject owner resolution, explicit owner overrides, ties, and every typed operation lane.
- Require an intent model for architecture-sensitive work; missing and materially unknown facts produce no design or scaffold recommendation.
- Exercise initialized, readable uninitialized, invalid canonical, blank-task, unreadable-input, read-only, and repository-operation cases. A `no-rms-change` report contains no design, spec, source-edit, gate, audit, or pending-candidate step.
- Compare clean and unrelated-dirty fixtures, compact and detailed projections, safe argument rendering, manual authorization steps, filesystem snapshots, and provider/prescribed-work sentinels.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked next_surface_projects_v2_and_repository_operations -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked next_selects_owner_deterministically_without_guessing -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked intent_model_requires_material_facts_and_forbids_architecture_fields -- --nocapture
```

Acceptance oracle:

- Every constructed report uses a declared result and lane, exits successfully, and preserves deterministic owner routing without guessing through ties.
- JSON has `schema: rms.surface/v2`, the shared surface fields, lane, confidence, owner state, and ordered typed steps. Command steps carry `program` plus `args`; manual commit steps carry `host-required` authorization and no Git program.
- Unrelated working-tree state and raw task tokens cannot change typed classification or structured-subject ownership. Construction performs no writes, verification, provider execution, or prescribed-work execution unless read-only provider extraction is explicitly requested and recorded.

Verification status: this file declares the executable proof protocol and does not assert an observed pass. Source provenance and executed results are resolved from the authorized candidate commit by `rms audit --root . --strict`.

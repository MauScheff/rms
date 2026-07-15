# Contract Evidence: explain module v2

Promise:

- `explain-module` constructs an answer-first deterministic explanation from the selected module's canonical artifacts without provider execution or project mutation.
- Default text and `rms.surface/v2` JSON contain one concise answer, at most three reasons, one immediate action, and completion conditions. Detailed canonical inventory is opt-in and remains nested under the same envelope.

Executable scenarios:

- Exercise focused-question, no-question overview, unsupported-question, explicit-module, inferred-module, and ambiguous-module cases.
- Assert focused sections appear once, unsupported questions return `insufficient-evidence`, and every constructed report exits successfully.
- Compare default, `--json`, `--details`, and `--details --json` projections; verify the versioned envelope and canonical evidence paths while default output omits inventories.
- Snapshot the fixture and provider sentinel before and after construction; neither changes and no provider process runs.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked explain_surface_is_answer_first_compact_and_deterministic -- --nocapture
```

Acceptance oracle:

- Stable canonical input produces a stable, nonduplicative answer whose claims are traceable to the selected module.
- An unreadable root or module, ambiguous inference, or unrepresentable result fails construction; insufficient canonical evidence is a truthful successful report.
- Machine-readable output has `schema: rms.surface/v2`, `command: explain`, the shared surface fields, `answer`, and canonical evidence paths. Details extend rather than replace that envelope.

Verification status: this file declares the executable proof protocol and does not assert an observed pass. Source provenance and executed results are resolved from the authorized candidate commit by `rms audit --root . --strict`.

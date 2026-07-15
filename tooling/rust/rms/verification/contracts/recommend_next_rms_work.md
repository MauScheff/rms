# Contract Evidence: recommend next RMS work

Promise:

- `recommend-next-rms-work` constructs a deterministic, prospective, read-only work prescription for a nonblank task.
- A constructed report uses only the declared result and repository-kind vocabularies, resolves ownership without guessing through ties, and keeps executable commands distinct from manual authorization steps.
- Report construction neither uses an unrelated Git diff as task evidence nor executes providers, verification, filesystem mutations, or Git commands.

Deterministic scenarios:

- Classify fixtures for `system-root`, `module-root`, `system-container`, `multi-system-workspace`, `module-workspace`, and `uninitialized`. Root-level system/context readiness is `not-applicable` only where repository shape makes those artifacts inapplicable; a partial canonical root remains missing or invalid.
- Resolve an explicit module, direct root module, sole top-level module, unique positive task match, and recursive composite route. Preserve ranked candidates and return `needs-owner` for a tie or no positive match.
- Classify read-only, design, semantic, surface, semantic-plus-surface, implementation-candidate, and undetermined task lanes from task text and canonical artifacts alone.
- Compare reports produced before and after an unrelated dirty-file fixture. Owner, lane, confidence, and prescribed steps remain equal.
- Serialize the same report model to text and JSON. Every executable step has `program`, `args`, and an independently escaped display value; a candidate commit is a manual authorization step and has no executable Git program.
- Snapshot the fixture tree and provider sentinel before and after report construction. Neither changes. Blank tasks and unreadable explicit inputs fail construction, while bootstrap, design, ambiguity, and blocked canonical reports remain successfully constructed.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked next_classifies_repository_kinds_and_not_applicable_artifacts -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked next_selects_owner_deterministically_without_guessing -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked next_classifies_prospective_task_lanes_independent_of_git_diff -- --nocapture
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked next_text_and_json_share_stable_safe_read_only_steps -- --nocapture
```

Acceptance oracle:

- Stable fixture input produces byte-stable ordering and equal semantic fields in text and JSON.
- Explicit ownership outranks inferred ownership; recursive routing terminates through a visited set; equal top scores and non-positive matches never select an owner.
- Dirty working-tree state cannot alter the prospective classification.
- Shell-significant task and path text remains one argument in `args`, while display escaping is presentation only.
- Successful construction performs no writes or child-process/provider execution and never represents a commit as an executable step.
- Construction failures are limited to blank task, unreadable requested input, or evidence that cannot be represented truthfully.

Verification status: this file declares the deterministic proof protocol and does not assert an observed pass. Source provenance and executed results are resolved from the authorized candidate commit by `rms audit --root . --strict`.

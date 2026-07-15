# Contract Evidence: describe RMS command surface

Promise:

- `describe-rms-command-surface` presents `init`, `next`, `explain`, `check`, and `view` as the five-command doorway, with the help meta-command, while specialist commands remain directly callable but absent from default help.
- Explicit all-command help derives a complete, stable, grouped catalog from the same command definitions used for parsing.

Executable scenarios:

- Snapshot default help and assert it exposes exactly the five primary commands plus help, without specialist command entries.
- Snapshot `rms help --all` and assert every registered specialist command appears exactly once in its functional group.
- Parse representative and exhaustive primary/specialist routes through the shared registry, including malformed and unsupported help selectors.
- Assert help construction reads no project artifacts, writes no files, and invokes no providers or child processes.

Command/tool:

```bash
cargo test --manifest-path tooling/rust/rms/Cargo.toml --locked help_surface_exposes_five_primary_and_grouped_expert_commands -- --nocapture
```

Acceptance oracle:

- Default help is the small conceptual surface; all-command help is the complete discovery surface; both are byte-stable for unchanged command metadata.
- Every command shown is parseable, every registered specialist command is discoverable through explicit all-command help, and neither help path changes command behavior.

Verification status: this file declares the executable proof protocol and does not assert an observed pass. Source provenance and executed results are resolved from the authorized candidate commit by `rms audit --root . --strict`.

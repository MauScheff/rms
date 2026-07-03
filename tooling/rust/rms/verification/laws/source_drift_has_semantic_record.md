# Law Evidence: source drift has semantic record

Promise:

- `source-drift-has-semantic-record`
- Source behavior changes require committed semantic-change evidence before strict production audit can pass.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml strict_audit_flags_placeholder_semantic_change_command_logs`
- `cargo test --manifest-path tooling/rust/rms/Cargo.toml strict_audit_flags_applied_semantic_change_not_reflected_in_machine`

Expected result:

- Strict audit rejects placeholder semantic-change commands and rejects applied change records that are not reflected in canonical manifests.

Source revision:

- Recorded by git commit before a production claim.

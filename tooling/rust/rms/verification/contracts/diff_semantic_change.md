# Contract Evidence: diff-semantic-change

Promise: `diff-semantic-change` exposes the current git diff for a semantic target so source-only behavior drift can be reviewed.

Scenario: `rms spec diff <module.yaml|implementation.yaml>` reports the target and relevant git diff as advisory evidence.

Command: `cargo test --manifest-path tooling/rust/rms/Cargo.toml impact git -- --nocapture`

Expected result: source changes are classified alongside semantic artifacts, and strict audit can fail source drift without a semantic delta.

Source revision: git:dfe027ab8502 plus current semantic-gate change under review.

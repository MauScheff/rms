# Contract Evidence: diff-machine-structure

Promise: `diff-machine-structure` exposes the current git diff for an implementation target without treating diff text as semantic authority.

Scenario: `rms machine diff <implementation.yaml>` reports target identity and changed source/canonical artifacts for review.

Command: `cargo test --manifest-path tooling/rust/rms/Cargo.toml impact git -- --nocapture`

Expected result: changed implementation paths are classified by git impact logic and remain review evidence until canonical artifacts are updated.

Source revision: git:dfe027ab8502 plus current semantic-gate change under review.

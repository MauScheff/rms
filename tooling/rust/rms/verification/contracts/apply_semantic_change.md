# Contract Evidence: apply-semantic-change

Promise: `apply-semantic-change` validates `rms/semantic-change/v0.1` and updates laws, contracts, machine declarations, runnable surface declarations, and evidence obligations together.

Scenario: valid YAML adds a law, public contract, machine transition, and evidence files; a law without evidence is rejected.

Command: `cargo test --manifest-path tooling/rust/rms/Cargo.toml spec_apply -- --nocapture`

Expected result: semantic changes are applied atomically enough for the target artifacts, while invalid semantic deltas fail with focused diagnostics.

Source revision: git:dfe027ab8502 plus current semantic-gate change under review.

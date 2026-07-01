# Contract Evidence: check-semantic-change

Promise: `check-semantic-change` reports semantic-first completeness for a module or implementation target.

Scenario: `rms spec check <module.yaml|implementation.yaml>` combines canonical law/contract/evidence checks with implementation structure checks when an implementation binding exists.

Command: `target/debug/rms spec check tooling/rust/rms/module.yaml`

Expected result: findings identify missing evidence, placeholder evidence, and structure gaps without mutating files.

Source revision: git:dfe027ab8502 plus current semantic-gate change under review.

# Explicit Reusable Intent

Law: `reusable-intent-is-explicit`

Promise: reusable capability and package-proof obligations follow explicit canonical reuse declarations only.

Scenario: a tool's `x-rms.last_semantic_intent` mentions reusable package verification, while `x-rms.reusable` and `x-rms.reuse.public_facade` are absent.

Command/tool: `cargo test --manifest-path tooling/rust/rms/Cargo.toml semantic_completeness_ignores_incidental_reuse_words_in_history`

Expected result: the tool receives neither `semantic.reusable-capability-missing` nor `semantic.reusable-package-evidence-missing`; explicitly reusable fixtures continue to receive both obligations when proof is absent.

Source provenance expectation: the passing test and this evidence file are committed together before strict production audit.

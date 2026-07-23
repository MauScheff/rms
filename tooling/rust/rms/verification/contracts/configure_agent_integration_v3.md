# Evidence: contract proves configure-agent-integration

Promise:

- configure-agent-integration

Scenario:

- Generate and synchronize managed agent guidance for supported integrations.
- Inspect the generated change-only start/route rule, including its native read-only exclusion, after a provider failure or non-ready ownership result.

Command/tool:

- `cargo test --manifest-path tooling/rust/rms/Cargo.toml agent --no-fail-fast`
- `rms check --changes --root . --module tooling/rust/rms/module.yaml`

Expected result:

- Generated guidance requires explicit `--ai` for software-change routing, keeps read-only and operational work native until a concrete change is proposed, reserves typed intent for intentional caller input, and forbids inferred owner selection from non-ready evidence.
- Managed asset and generated-distribution digests remain synchronized.

Source revision: recorded by git commit or strict audit provenance before production use.

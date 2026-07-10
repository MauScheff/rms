---
name: compose-modules
description: Check or design the composition of RMS modules by matching capabilities, contracts, operational semantics, service constraints, effects, dependency direction, and conformance evidence.
---

# Compose Modules

1. Run `rms compose --root <root>` when the CLI is available.
2. Identify the consumer requirements and candidate provider capabilities.
3. Compare contract versions, meaning, preconditions, failures, authorization, and compatibility policy.
4. For reusable modules, confirm the provider declares `provides.capabilities[]`, the consumer declares `requires.capabilities[]` with the provider contract, and code imports only the provider's RMS public facade or calls a contract-shaped entrypoint. Do not treat native package exports as semantic authority when they bypass the facade.
5. Compare operational semantics: idempotency, ordering, consistency, timeout, retry, concurrency, compensation, and reconciliation. For cross-module effects, require an explicit request/result protocol; individual outcomes that affect later decisions must return through the consumer's canonical transition rather than remain hidden in an executor.
6. Check declared service constraints that the consumer depends on.
7. Confirm the host permits the provider's effects and required capabilities.
8. Check dependency direction and reject forbidden or ownership-breaking cycles.
9. Confirm the provider passes the required conformance suite.
10. When replacing a stateful implementation, verify export, migration, coexistence, rollback, and cutover behavior.
11. Produce a composition result listing satisfied, incompatible, unresolved, and not-applicable requirements.

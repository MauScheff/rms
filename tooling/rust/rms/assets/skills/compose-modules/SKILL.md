---
name: compose-modules
description: Check or design the composition of RMS modules by matching capabilities, contracts, operational semantics, service constraints, effects, dependency direction, and conformance evidence.
---

# Compose Modules

1. Run `rms compose --root <root>` when the CLI is available.
2. Identify the consumer requirements and candidate provider capabilities.
3. Compare contract versions, meaning, preconditions, failures, authorization, and compatibility policy.
4. Compare operational semantics: idempotency, ordering, consistency, timeout, retry, concurrency, compensation, and reconciliation.
5. Check declared service constraints that the consumer depends on.
6. Confirm the host permits the provider's effects and required capabilities.
7. Check dependency direction and reject forbidden or ownership-breaking cycles.
8. Confirm the provider passes the required conformance suite.
9. When replacing a stateful implementation, verify export, migration, coexistence, rollback, and cutover behavior.
10. Produce a composition result listing satisfied, incompatible, unresolved, and not-applicable requirements.

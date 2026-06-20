---
name: compose-modules
description: Check or design the composition of RMS modules by matching capabilities, contracts, operational semantics, service constraints, effects, dependency direction, and conformance evidence.
---

# Compose Modules

1. Identify the consumer requirements and candidate provider capabilities.
2. Compare contract versions, meaning, preconditions, failures, authorization, and compatibility policy.
3. Compare operational semantics: idempotency, ordering, consistency, timeout, retry, concurrency, compensation, and reconciliation.
4. Check declared service constraints that the consumer depends on.
5. Confirm the host permits the provider's effects and required capabilities.
6. Check dependency direction and reject forbidden or ownership-breaking cycles.
7. Confirm the provider passes the required conformance suite.
8. When replacing a stateful implementation, verify export, migration, coexistence, rollback, and cutover behavior.
9. Produce a composition result listing satisfied, incompatible, unresolved, and not-applicable requirements.

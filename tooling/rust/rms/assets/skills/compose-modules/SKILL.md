---
name: compose-modules
description: Check or design the composition of RMS modules by matching capabilities, contracts, operational semantics, service constraints, effects, dependency direction, and conformance evidence.
---

# Compose Modules

1. Run `rms compose --root <root>` when the CLI is available.
2. Identify the consumer requirements and candidate provider capabilities.
3. Compare contract versions, meaning, preconditions, failures, authorization, and compatibility policy.
4. Compose versioned artifact requirements against exactly one compatible provider. For public conversations, require one shared protocol automaton, one implementation owner per participant, and exactly one sender and receiver mapping per message.
5. For reusable modules, confirm the provider declares `provides.capabilities[]` with a capability-kind contract, the consumer declares `requires.capabilities[]` with the same stable capability identity and exact contract reference, and code imports only the provider's RMS public facade or calls a contract-shaped entrypoint. Keep a provider command and its reusable capability as distinct contract identities. The consumer must have one `dependency_behavior_bindings` entry naming its exact local `path#symbol` consumer and the matching provider module/contract; the provider must close the capability through one `public_behavior_bindings` entry. Do not treat native package exports as semantic authority when they bypass the facade.
6. Compare operational semantics: idempotency, ordering, consistency, timeout, retry, concurrency, compensation, and reconciliation. For cross-module effects, require an explicit request/result protocol; individual outcomes that affect later decisions must return through the consumer's canonical transition rather than remain hidden in an executor.
7. Check declared service constraints that the consumer depends on.
8. Confirm the host permits the provider's effects and required capabilities.
9. Check dependency direction and reject forbidden or ownership-breaking cycles.
10. Confirm the provider passes the required conformance suite.
11. When a composite parent repeats an exported child law, use `verification.delegations` rather than inventing a duplicate parent property. Confirm the parent law, contained provider, provider law, provider property, public export, and concrete parent evidence all resolve through `rms compose`.
12. For cross-module temporal promises, declare composition-scoped observations over public messages, transitions, and trace metrics; evaluate stitched or probe-system traces, preserve causation in explanations, and require exhausted search for universal finite conclusions.
13. When replacing a stateful implementation, verify export, migration, coexistence, rollback, and cutover behavior.
14. When a scenario spans modules, stitch execution-derived trace bundles and require message identity, correlation, causation, source, target, and sequence to survive each handoff. Diagnose the first broken handoff rather than inferring it from local traces.
15. Produce a composition result listing satisfied, incompatible, unresolved, and not-applicable requirements.

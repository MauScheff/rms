---
name: evolve-contract
description: Change an RMS public contract safely; use for commands, queries, events, APIs, capabilities, schemas, failure semantics, or operational behavior consumed outside a module.
---

# Evolve a Public Contract

1. Identify all published contract versions and known consumers.
2. Classify the proposed change:
   - implementation-only;
   - backward-compatible additive;
   - behavioral but compatible;
   - deprecated;
   - breaking shape change;
   - breaking semantic or operational change;
   - stored-state or migration change.
3. Compare not only schema but also:
   - meaning;
   - preconditions and postconditions;
   - failures;
   - authorization;
   - idempotency;
   - ordering;
   - consistency;
   - timeout and retry behavior.
4. Preserve the existing version when compatibility can be maintained cleanly.
5. Introduce a new version for breaking changes.
6. Define migration, coexistence, translation, and deprecation behavior.
7. Update provider and consumer contract evidence.
8. Run compatibility tooling and relevant scenarios.
9. Record the decision and consumer impact in a concise change note.

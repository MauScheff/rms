---
name: evolve-contract
description: Change an RMS public contract safely; use for commands, queries, events, APIs, capabilities, schemas, failure semantics, or operational behavior consumed outside a module.
---

# Evolve a Public Contract

1. Run the `inspect-module` workflow for the owning module.
2. Build a bounded packet with `rms context <module> --task "<task>"` when the CLI is available. Use `rms evolve-contract <module> --task "<task>"` when a compatibility prompt would help.
3. Identify all published contract versions and known consumers.
4. Classify the proposed change:
   - implementation-only;
   - backward-compatible additive;
   - behavioral but compatible;
   - deprecated;
   - breaking shape change;
   - breaking semantic or operational change;
   - stored-state or migration change.
5. Compare not only schema but also:
   - meaning;
   - preconditions and postconditions;
   - failures;
   - authorization;
   - idempotency;
   - ordering;
   - consistency;
   - timeout and retry behavior.
6. Preserve the existing version when compatibility can be maintained cleanly.
7. Introduce a new version for breaking changes.
8. Define migration, coexistence, translation, and deprecation behavior.
9. Update provider and consumer contract evidence.
10. Run `rms check-compat <old-module> <new-module>`, `rms validate --root <root>`, and relevant scenarios.
11. Record the decision and consumer impact in a concise change note.

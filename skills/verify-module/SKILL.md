---
name: verify-module
description: Verify that an RMS module and its changes satisfy declared laws, contracts, scenarios, boundaries, profiles, dependency rules, and compatibility promises.
---

# Verify an RMS Module

1. Read the target manifest and implementation binding.
2. Confirm manifest validity and referenced-file existence.
3. Check ownership and dependency boundaries:
   - no undeclared imports or calls;
   - no private-state access;
   - no undeclared effects;
   - no business concepts added to the technical kernel without justification.
4. Run declared evidence:
   - laws;
   - contracts;
   - scenarios;
   - boundaries when applicable.
5. Check all declared profile obligations:
   - Stateful: transitions, concurrency, persistence, migration;
   - Distributed: idempotency, delivery, retry, timeout, duplicates, reconciliation;
   - Workflow: terminal states, deadlines, compensation, resumption;
   - Boundary: validation, trust, limits, compatibility.
6. Check public compatibility against the previous accepted version.
7. Confirm manifests, glossary, contracts, and operational docs remain accurate.
8. Produce an evidence summary with pass, fail, skipped, and not-applicable items. Do not report success without identifying the checks actually run.

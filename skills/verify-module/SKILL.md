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
5. Check domain representation evidence:
   - closed domain alternatives use ADTs, sealed variants, enums, or an equivalent explicit representation;
   - values with validity rules cannot be constructed invalidly except through rejected boundary input;
   - expected domain failures are represented explicitly enough for callers to handle;
   - untrusted or versioned input is validated before domain entry;
   - lifecycle/order-dependent behavior has a state model, transition table, or transition function.
6. Check negative cases. Verification should reject or make unrepresentable impossible variants, invalid constructors, malformed boundary input, and illegal state transitions.
7. Check all declared profile obligations:
   - Stateful: transitions, concurrency, persistence, migration;
   - Distributed: idempotency, delivery, retry, timeout, duplicates, reconciliation;
   - Workflow: terminal states, deadlines, compensation, resumption;
   - Boundary: validation, trust, limits, compatibility.
8. Check public compatibility against the previous accepted version.
9. Confirm manifests, glossary, contracts, and operational docs remain accurate.
10. Produce an evidence summary with pass, fail, skipped, and not-applicable items. Do not report success without identifying the checks actually run.

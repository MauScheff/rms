---
name: verify-module
description: Verify that an RMS module and its changes satisfy declared laws, contracts, scenarios, boundaries, profiles, dependency rules, and compatibility promises.
---

# Verify an RMS Module

1. Run `rms diagnose` when starting from an unfamiliar checkout.
2. Read the target manifest and implementation binding.
3. Run `rms validate --root <root>` or validate the explicit target manifests.
4. Run `rms review <module>` when verifying an active diff. Run `rms verify <implementation.yaml>` when the implementation binding declares `commands.verify`.
5. Confirm manifest validity and referenced-file existence.
6. Check ownership and dependency boundaries:
   - no undeclared imports or calls;
   - no private-state access;
   - no undeclared effects;
   - no business concepts added to the technical kernel without justification.
7. Run declared evidence:
   - laws;
   - contracts;
   - scenarios;
   - boundaries when applicable.
8. Check domain representation evidence:
   - closed domain alternatives use ADTs, sealed variants, enums, or an equivalent explicit representation;
   - values with validity rules cannot be constructed invalidly except through rejected boundary input;
   - expected domain failures are represented explicitly enough for callers to handle;
   - untrusted or versioned input is validated before domain entry;
   - public read models or result structs without public constructors are declared in `architecture.allowed_missing_constructors` only when they are produced by a named query/projector with evidence;
   - lifecycle/order-dependent behavior has a state model, transition table, or transition function.
9. Check negative cases. Verification should reject or make unrepresentable impossible variants, invalid constructors, malformed boundary input, and illegal state transitions.
10. Check all declared profile obligations:
   - Stateful: transitions, concurrency, persistence, migration;
   - Distributed: idempotency, delivery, retry, timeout, duplicates, reconciliation;
   - Workflow: terminal states, deadlines, compensation, resumption;
   - Boundary: validation, trust, limits, compatibility.
11. Check public compatibility against the previous accepted version with `rms check-compat` when manifests changed.
12. Confirm manifests, glossary, contracts, and operational docs remain accurate.
13. Produce an evidence summary with pass, fail, skipped, and not-applicable items. Do not report success without identifying the checks actually run.

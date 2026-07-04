---
name: verify-module
description: Verify that an RMS module and its changes satisfy declared laws, contracts, scenarios, boundaries, profiles, dependency rules, and compatibility promises.
---

# Verify an RMS Module

1. Run `rms diagnose` when starting from an unfamiliar checkout.
2. Read the target manifest and implementation binding.
3. Run `rms validate --root <root>` or validate the explicit target manifests.
4. Run `rms review <module>` when verifying an active diff. Run `rms spec check <module.yaml|implementation.yaml>` and `rms machine check <implementation.yaml>` when an implementation binding exists. Run `rms verify <implementation.yaml>` when the implementation binding declares `commands.verify`, or `rms verify <composite-module.yaml>` for composite rollups. Run `rms audit --root <root> --strict` before claiming production-ready RMS software.
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
   - local trace bundles with `rms trace check`, `rms trace replay`, or `rms trace diagnose` when transition evidence is recorded as JSON or YAML.
8. Check domain representation evidence:
   - closed domain alternatives use ADTs, sealed variants, enums, or an equivalent explicit representation;
   - values with validity rules cannot be constructed invalidly except through rejected boundary input;
   - expected domain failures are represented explicitly enough for callers to handle;
   - untrusted or versioned input is validated before domain entry;
   - public read models or result structs without public constructors are declared in `architecture.allowed_missing_constructors` only when they are produced by a named query/projector with evidence;
   - lifecycle/order-dependent behavior has a state model, transition table, or transition function;
   - traceable behavior declares message envelopes, transition output, transition records, replay support, and first-bad-transition evidence where applicable.
   - semantic roles, state variants, commands, events, effects, effect results, public entrypoints, and evidence roles are declared in canonical artifacts rather than invented by direct source edits.
   - public commands in `module.yaml` are represented by the declared implementation machine, parser, adapter, transition, representation, or semantic functions.
   - runnable app, browser, UI, and CLI surfaces route through declared public entrypoints, parsers, adapters, or boundary machines instead of importing pure/private decision roles directly.
   - generated `Accept`/`Reject` scaffolds have been replaced or semantically justified when the module publishes product-specific commands.
9. Check negative cases. Verification should reject or make unrepresentable impossible variants, invalid constructors, malformed boundary input, and illegal state transitions.
10. Check all declared profile obligations:
   - Stateful: transitions, concurrency, persistence, migration;
   - Distributed: idempotency, delivery, retry, timeout, duplicates, effect lifecycle, reconciliation;
   - Workflow: terminal states, deadlines, subscription registry, golden timeline, replay bundle, compensation, resumption;
   - Boundary: validation, trust, limits, compatibility.
11. Check public compatibility against the previous accepted version with `rms check-compat` when manifests changed.
12. Confirm manifests, glossary, contracts, and operational docs remain accurate.
13. Produce an evidence summary with pass, fail, skipped, and not-applicable items. For stateful or workflow behavior, name whether transition records, golden timeline tests, replay bundles, `rms trace` checks, first-bad-transition diagnostics, and strict audit were checked. Do not report success without identifying the checks actually run, and do not treat implemented modules as complete while `rms validate --root <root>` reports `evidence.placeholder`, `evidence.bootstrap-active`, `evidence.source-unpinned`, `evidence.semantic-shape-only`, `structure.public-command-not-represented`, `structure.generic-scaffold-command-active`, or `structure.runnable-surface-bypasses-boundary` for those modules.

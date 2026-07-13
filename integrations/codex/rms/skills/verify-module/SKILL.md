---
name: verify-module
description: Verify that an RMS module and its changes satisfy declared laws, contracts, scenarios, boundaries, profiles, dependency rules, and compatibility promises.
---

# Verify an RMS Module

1. Run `rms diagnose` when starting from an unfamiliar checkout.
2. Read the target manifest and implementation binding.
3. Run `rms validate --root <root>` or validate the explicit target manifests.
4. Run `rms review <module>` when verifying an active diff. Run `rms spec check <module.yaml|implementation.yaml>`, `rms machine check <implementation.yaml>` when an implementation binding exists, and `rms property check <module.yaml|implementation.yaml>` when laws, parsers, numeric bounds, reusable modules, generated input spaces, fuzz targets, or counterexamples are involved. Run `rms verify <implementation.yaml>` when the implementation binding declares `commands.verify`, or `rms verify <composite-module.yaml>` for composite rollups. Then require `rms gate --root <root>` to exit zero, commit the candidate, and require `rms audit --root <root> --strict` to exit zero before claiming production-ready RMS software. A failed check is a failure, not a manual note.
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
   - semantic properties and fuzz targets with `rms property check`;
   - property/fuzz commands with `rms property run <implementation.yaml> --profile smoke` when the binding declares them;
   - recorded counterexamples with `rms property replay <counterexample.yaml>`.
   - local trace bundles with `rms trace check`, `rms trace replay`, or `rms trace diagnose` when transition evidence is recorded as JSON or YAML.
8. Check domain representation evidence:
   - closed domain alternatives use ADTs, sealed variants, enums, or an equivalent explicit representation;
   - values with validity rules cannot be constructed invalidly except through rejected boundary input;
   - expected domain failures are represented explicitly enough for callers to handle;
   - untrusted or versioned input is validated before domain entry;
   - public read models or result structs without public constructors are declared in `architecture.allowed_missing_constructors` only when they are produced by a named query/projector with evidence;
   - lifecycle/order-dependent behavior has a state model, transition table, or transition function;
   - traceable behavior declares message envelopes, transition output, transition records, replay support, and first-bad-transition evidence where applicable; an applied transition declaration is not itself replay evidence.
   - machine semantic lists contain actual alternatives while binding container names appear only under `architecture.machine.types`.
   - stateful machines have one state-plus-input transition; every transition input is exactly one command, observed event, or effect result.
   - every transition has a stable semantic case; every declared case occurs in declared transition source, no source-only branch escapes canonical semantics, every lifecycle state is reachable from `initial_state`, and each case appears in replay evidence whose provenance names that source file and branch.
   - every effect declares request/result atomicity, an exact executor symbol, and an effectful `effect-executor` semantic function; every effectful stateful machine declares exact driver and transition-record functions; every effect-emitting runnable surface reaches that driver; the driver retains complete records, advances from `state_after`, executes `output.effects`, and owns the complete repeated cycle; and surfaces, adapters, and executors contain no hidden business sequencing or state progression.
   - every declared message envelope has a binding-native representation, and transition arithmetic over indices, counts, attempts, offsets, lengths, and sequences is checked or bounded with explicit rejection for extreme inputs.
   - broad laws that say always, never, bounded, ordered, normalized, parsed, generated, impossible, or must not happen have semantic properties with input spaces, operations, oracles, evidence, and replayable counterexample policy.
   - boundary, storage, integration, and runnable parser surfaces have fuzz-style semantic targets or a concrete no-fuzz justification; fixed corpora do not satisfy open-ended fuzz claims, every non-corpus realization resolves an exact binding `path#symbol` harness, and untouched generated property evidence is rejected as an obligation rather than proof.
   - semantic roles, state variants, commands, events, effects, effect results, semantic-function authority bindings, public entrypoints, and evidence roles are declared in canonical artifacts rather than invented by direct source or manifest edits; active non-composition invariants have a matching function owner and active semantic-change records reflect the exact binding.
   - public commands in `module.yaml` are represented by the declared implementation machine, parser, adapter, transition, representation, or semantic functions.
   - runnable app, browser, UI, CLI, HTTP, batch, and executable surfaces are declared and route through declared public entrypoints, parsers, adapters, or boundary machines instead of importing or duplicating pure/private decision roles directly.
   - runnable delegation resolves to a declared role or concrete symbol and declares boundary effects or a no-effect justification.
   - public domain structs cannot bypass validated construction through `allowed_public_field_structs`; only envelopes and transition/provenance records use that exemption.
   - strict audit reports `semantic.revision-integrity`; a clean Git commit alone is not proof when canonical semantics changed outside RMS apply.
   - reusable modules declare `provides.capabilities[]`, expose one RMS public facade, include package/reuse evidence, and have no consumers importing private representation, transition, parser, adapter, or port internals.
   - generated public contracts have been replaced through `contracts.set` with product-specific meaning, accepted inputs, guaranteed outcomes, and rejection categories.
   - generated `Accept`/`Reject` scaffolds have been replaced or semantically justified when the module publishes product-specific commands.
9. Check negative cases. Verification should reject or make unrepresentable impossible variants, invalid constructors, malformed boundary input, and illegal state transitions.
10. Check all declared profile obligations:
   - Stateful: transitions, concurrency, persistence, migration;
   - Distributed: idempotency, delivery, retry, timeout, duplicates, effect lifecycle, reconciliation;
   - Workflow: terminal states, deadlines, subscription registry, golden timeline, replay bundle, compensation, resumption;
   - Boundary: validation, trust, limits, compatibility.
11. Check public compatibility against the previous accepted version with `rms check-compat` when manifests changed.
12. Confirm manifests, glossary, contracts, and operational docs remain accurate.
13. For reusable modules, run `rms package <module.yaml>` and confirm it records a concrete pass in the declared package evidence, rebuilds, and verifies the final artifact. Use `rms verify-package <package-dir>` as an independent recheck, then confirm the package contains the module manifest, contracts, implementation binding, declared public facade, recorded evidence, conformance report, source revision, and checksums.
14. Produce an evidence summary with pass, fail, skipped, and not-applicable items. For stateful or workflow behavior, name whether transition records, golden timeline tests, replay bundles, `rms trace` checks, first-bad-transition diagnostics, semantic properties, counterexample replay, and strict audit were checked. Do not report success without identifying the checks actually run, and do not treat implemented modules as complete while `rms validate --root <root>` reports `semantic.contract-scaffold-active`, `evidence.placeholder`, `evidence.bootstrap-active`, `evidence.source-unpinned`, `evidence.semantic-shape-only`, `semantic.reusable-capability-missing`, `semantic.reusable-package-evidence-missing`, `semantic.property-without-input-space`, `semantic.property-without-oracle`, `semantic.property-evidence-missing`, `structure.property-target-missing`, `structure.boundary-parser-without-fuzz-property`, `structure.public-command-not-represented`, `structure.generic-scaffold-command-active`, `structure.native-package-export-mismatch`, or `structure.runnable-surface-*` for those modules. Evidence must not claim a current filesystem snapshot or missing Git revision; strict audit resolves the committed candidate revision. Never repair canonical artifacts by direct editing; if the CLI cannot express a required repair, report an RMS product gap.

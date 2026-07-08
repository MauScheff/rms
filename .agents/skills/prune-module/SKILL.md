---
name: prune-module
description: Remove semantically unnecessary RMS code, tests, fixtures, docs, helpers, abstractions, and compatibility residue while preserving public contracts, ownership, effects, and evidence.
---

# Prune an RMS Module

Use this skill when the requested outcome is less accidental complexity, lower technical debt, or removal of artifacts that may have accumulated during testing, development, migration, or experimentation.

The governing test is semantic reachability:

```text
Every retained artifact should serve a current manifest promise, public contract, invariant, declared effect, profile obligation, compatibility policy, operational recovery path, implementation binding, or verification evidence.
```

1. Run the `inspect-module` workflow for the owning module.
2. Build a bounded packet with `rms context <module> --task "<task>"` when the CLI is available.
3. State the public semantics that must be preserved:
   - purpose and ownership;
   - public commands, queries, events, APIs, and capabilities;
   - invariants and laws;
   - declared effects and required capabilities;
   - profiles and operational semantics;
   - compatibility policy, deprecations, and active consumers;
   - verification evidence and operational recovery paths.
4. Build a semantic root set from the canonical artifacts:
   - `system.yaml`, `context-map.yaml`, and the target `module.yaml`;
   - public contracts, invariants, and schemas;
   - glossary entries and active linked decisions;
   - implementation binding and declared source roots;
   - verification evidence, runbooks, migrations, and compatibility material.
5. Inventory candidate artifacts inside the owning boundary:
   - implementation files and internal helpers;
   - tests, fixtures, snapshots, generated files, and scripts;
   - adapters, shims, migrations, and compatibility branches;
   - docs, examples, comments, and diagrams;
   - dependencies, feature flags, configuration, and build targets.
6. Classify each candidate by its current semantic role:
   - public semantic surface;
   - private implementation of a declared promise;
   - verification evidence;
   - operational or recovery support;
   - implementation-binding or adapter support;
   - compatibility shim with named consumers and removal condition;
   - generated artifact with reproducible source;
   - candidate residue.
7. Treat the following as pruning signals:
   - abstractions introduced only to satisfy an old test shape;
   - helpers, fixtures, or snapshots no longer tied to declared evidence;
   - duplicate terms for the same domain concept;
   - compatibility code without a policy, consumer, or expiry condition;
   - generated artifacts that cannot be regenerated or validated;
   - state/status fields without lifecycle-dependent behavior;
   - folders, modules, or classes created around nouns rather than ownership, invariants, contracts, or replaceability;
   - speculative extension points not tied to an active decision;
   - comments or docs that restate obsolete behavior.
8. Prefer deletion or inlining before introducing a replacement abstraction.
9. When pruning compatibility material, first confirm the compatibility policy, active consumers, stored-state impact, migrations, and rollback or recovery path.
10. When pruning verification material, preserve enough positive and negative evidence to demonstrate the declared promises:
   - laws and invariants still hold;
   - public contract behavior remains compatible;
   - invalid constructors, malformed boundary input, impossible variants, and illegal transitions are rejected or unrepresentable when applicable.
11. Do not hide a semantic change as pruning. If removal changes public meaning, switch to `evolve-contract` or `implement-change` and make compatibility impact explicit.
12. Run `rms validate --root <root>` and the implementation binding's build and verification commands.
13. Summarize:
    - preserved public semantics;
    - deleted, merged, inlined, or renamed artifacts;
    - artifacts intentionally retained and their semantic role;
    - compatibility, migration, or operational impact;
    - verification evidence;
    - remaining residue and explicit removal conditions.

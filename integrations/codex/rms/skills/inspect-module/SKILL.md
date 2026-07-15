---
name: inspect-module
description: Inspect an RMS module before planning or changing it; use when ownership, boundaries, dependencies, effects, or reliability obligations are unclear.
---

# Inspect an RMS Module

1. Run `rms check --environment` once per session when the CLI is available.
2. Run `rms explain --module <module>` for a human-readable module doorway. If there is a specific uncertainty, pass it as the optional question; add `--details` only when the compact answer is insufficient.
3. Run `rms inspect <module>` or `rms context <module> --task "<task>"` before planning a change.
4. Identify the system, bounded context, and target module.
5. Read, in order:
   - the system manifest and context map;
   - the target `module.yaml`;
   - applicable glossary entries;
   - public contracts;
   - direct dependency contracts;
   - the implementation binding and relevant decisions.
6. Do not read unrelated implementation unless the public artifacts are insufficient. Never inspect sibling projects, prior dogfood runs, RMS source, or generated examples outside the target project to infer a semantic-change schema or borrow product semantics; use the rendered RMS prompt and deterministic diagnostics.
7. Produce a concise module brief:
   - purpose and ownership;
   - public surface;
   - required capabilities;
   - declared profiles;
   - invariants;
   - effects and operational semantics;
   - compatibility policy;
   - verification evidence;
   - each public command/query/capability's exact contract -> semantic function -> classified machine cases -> evidence path;
   - each required capability's exact local consumer and module-provider contract or explicit external resolution;
   - representation obligations for closed variants, validated values, boundary schemas, and lifecycle state;
   - suspected gaps or drift.
   - binding type mappings versus actual semantic alternatives;
   - canonical input categories and effect request/result protocols;
   - exact machine driver, transition-record, and effect executor symbols, plus whether live execution retains complete records and each effect-emitting runnable surface reaches that driver and leaves the complete repeated transition/effect/result cycle inside it even when public and machine command names differ;
   - declared transition cases versus source branches, lifecycle-state reachability from `initial_state`, whether expected failures remain in a typed transition rejection channel, and whether execution-derived traces name the transition source and match each case's exact outputs rather than copying declarations;
   - whether any executor hides sequencing, retry, compensation, or state progression.
   - declared artifacts and transformations, including version and compatibility ownership;
   - public protocol participants/message mappings and whether composition closes every route;
   - resource protocols and terminal-path closure;
   - privileged, unsafe, or foreign authority bindings and exact safe facades;
   - temporal properties and whether each realization can prove its declared scope.
8. For a proposed task, identify the owning module and the smallest affected contract surface.
9. Identify whether the task should use:
   - an ADT, sealed variant, or enum for closed alternatives;
   - a validated constructor or opaque type for invalid raw values;
   - a schema or validator for boundary input;
   - a state model only when legal behavior depends on lifecycle or order.
   - a typed effect result and follow-up transition when an external outcome changes what happens next.
10. Flag any need to cross a private boundary rather than silently doing so.

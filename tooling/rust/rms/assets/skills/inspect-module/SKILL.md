---
name: inspect-module
description: Inspect an RMS module before planning or changing it; use when ownership, boundaries, dependencies, effects, or reliability obligations are unclear.
---

# Inspect an RMS Module

1. Run `rms diagnose` once per session when the CLI is available.
2. Run `rms explain <module>` for a human-readable module doorway. If there is a specific uncertainty, pass it as the optional question.
3. Run `rms inspect <module>` or `rms context <module> --task "<task>"` before planning a change.
4. Identify the system, bounded context, and target module.
5. Read, in order:
   - the system manifest and context map;
   - the target `module.yaml`;
   - applicable glossary entries;
   - public contracts;
   - direct dependency contracts;
   - the implementation binding and relevant decisions.
6. Do not read unrelated implementation unless the public artifacts are insufficient.
7. Produce a concise module brief:
   - purpose and ownership;
   - public surface;
   - required capabilities;
   - declared profiles;
   - invariants;
   - effects and operational semantics;
   - compatibility policy;
   - verification evidence;
   - representation obligations for closed variants, validated values, boundary schemas, and lifecycle state;
   - suspected gaps or drift.
8. For a proposed task, identify the owning module and the smallest affected contract surface.
9. Identify whether the task should use:
   - an ADT, sealed variant, or enum for closed alternatives;
   - a validated constructor or opaque type for invalid raw values;
   - a schema or validator for boundary input;
   - a state model only when legal behavior depends on lifecycle or order.
10. Flag any need to cross a private boundary rather than silently doing so.

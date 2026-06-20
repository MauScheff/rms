---
name: inspect-module
description: Inspect an RMS module before planning or changing it; use when ownership, boundaries, dependencies, effects, or reliability obligations are unclear.
---

# Inspect an RMS Module

1. Identify the system, bounded context, and target module.
2. Read, in order:
   - the system manifest and context map;
   - the target `module.yaml`;
   - applicable glossary entries;
   - public contracts;
   - direct dependency contracts;
   - the implementation binding and relevant decisions.
3. Do not read unrelated implementation unless the public artifacts are insufficient.
4. Produce a concise module brief:
   - purpose and ownership;
   - public surface;
   - required capabilities;
   - declared profiles;
   - invariants;
   - effects and operational semantics;
   - compatibility policy;
   - verification evidence;
   - suspected gaps or drift.
5. For a proposed task, identify the owning module and the smallest affected contract surface.
6. Flag any need to cross a private boundary rather than silently doing so.

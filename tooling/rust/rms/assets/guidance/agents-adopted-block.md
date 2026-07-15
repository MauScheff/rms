<!-- RMS managed guidance: begin -->
## RMS Integration

Existing project instructions remain authoritative outside this managed section. Canonical RMS artifacts own semantics and architecture; agents fill declared role bodies.

- Begin with `rms next --task "<intent>" --root .` and follow its owner, context, declaration, verification, and completion prescription.
- For a new or adopted system, use `rms init [--adopt]` → authorized bootstrap commit → `rms design` → recommended scaffold.
- The pre-commit bootstrap state is exactly `bootstrap prepared; provenance baseline pending authorized commit`.
- Do not hand-edit RMS manifests, contracts, semantic functions, machine cases, surfaces, or evidence declarations. Use the prescribed RMS apply command, dry-run first.
- Keep pure roles pure, boundary IO in declared effects, cross-module access on public facades, and runnable surfaces delegated to exact declared callables.
- Finish with focused proof → `rms gate --root .` → authorized candidate commit → `rms audit --root . --strict`.
- Without commit authority, stop at exactly `candidate prepared; strict audit pending authorized commit`.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness.

<!-- RMS managed guidance: end -->

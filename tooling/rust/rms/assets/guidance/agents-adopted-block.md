<!-- RMS managed guidance: begin -->
## RMS Integration

Project instructions outside this section remain authoritative. Canonical RMS artifacts own semantics; agents fill declared roles.

- Extract typed semantic facts, never topology; start with `rms next "<intent>" --root . --intent-yaml '<rms/intent-model/v0.1>'` or recorded read-only `--ai` extraction.
- New or adopted systems follow `rms init [--adopt]` → authorized bootstrap commit → typed `rms design` → exactly one recommended scaffold.
- Without commit authority, stop at exactly `bootstrap prepared; provenance baseline pending authorized commit`.
- Apply canonical semantic or surface changes before source edits, dry-run first; do not hand-edit RMS declarations.
- Publish standalone capabilities through `rms spec apply`; use `rms add-capability-tree` only for an explicitly recommended recursive topology.
- Keep pure roles pure, IO in declared effects, cross-module access on public facades, and runnable surfaces delegated to exact callables.
- Finish with focused proof → `rms check --changes --root .` → authorized candidate commit → `rms check --committed --root .`.
- Without commit authority, stop at exactly `candidate prepared; strict audit pending authorized commit`.
- Progressive checks certify RMS module closures only, not unrelated repository paths.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness.

<!-- RMS managed guidance: end -->

<!-- RMS managed guidance: begin -->
## RMS Integration

Project instructions outside this section remain authoritative. Canonical RMS artifacts own semantics; agents fill declared roles.

- Begin work that requests or may require a software change with `rms next "<exact change task>" --root . --ai`. Skip RMS for read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, and discussion that requests no change; use native project tools. If an investigation reveals a proposed change, stop before editing and run `rms next` with that exact change task. Use typed intent flags only for CI, offline, or intentionally pre-structured caller input. Never use synthesized typed intent as an automatic provider-failure fallback. A non-ready or ownerless route selects no owner: do not infer one from candidates, context, neighboring modules, or implementation language.
- Pass the ready route receipt through `--route-receipt` on prescribed canonical mutators, including dry-runs; it grants neither source-edit nor Git authority.
- New or adopted systems follow `rms init [--adopt]` → authorized bootstrap commit → typed `rms design` → exactly one recommended scaffold.
- Without commit authority, stop at exactly `bootstrap prepared; provenance baseline pending authorized commit`.
- Apply canonical semantic or surface changes before source edits, dry-run first; do not hand-edit RMS declarations.
- Write normative semantic prose using ASD-STE100 rules as the controlled-language baseline. State one obligation, condition, or result in each sentence; name the subject explicitly; use one canonical term for each concept; preserve bounded-context terms, exact identifiers, schema keys, formulas, and quoted contract language; and give semantic precision and compatibility priority.
- Publish standalone capabilities through `rms spec apply`; use `rms add-capability-tree` only for an explicitly recommended recursive topology.
- Keep pure roles pure, IO in declared effects, cross-module access on public facades, and runnable surfaces delegated to exact callables.
- Temporal promises use typed observations, explicit assumptions, closed expressions, and dimensionally valid quantities; bounded exploration is inconclusive unless exhausted.
- Derive strong verification lanes from risk. Inspect unattended work with `rms hunt --dry-run`; run it from a clean commit, require replayable behavioral findings, and never call bounded evidence bug-free.
- Finish local work with applicable probe-first focused proof → coverage-aware `rms check --changes --root .` → authorized candidate commit → coverage-aware `rms check --committed --root .`.
- Run returned project-owned native proof commands. Treat outside-coverage paths as typed handoffs, not RMS-certified work or automatic adoption requests.
- Use `rms check --all --root .` for exhaustive release or CI certification. Keep committed provenance evidence and strict full-repository gates intact.
- Without commit authority, stop at exactly `candidate prepared; strict audit pending authorized commit`.
- Affected checks certify selected RMS module closures only. They keep candidate regressions, unchanged baseline debt, native paths, and outside-coverage paths distinct.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness.

<!-- RMS managed guidance: end -->

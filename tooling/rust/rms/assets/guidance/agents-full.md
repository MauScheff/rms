# Agent Instructions

<!-- RMS generated full guidance -->

## Authority

This repository follows Reliable Modular Systems. Canonical RMS artifacts own architecture, behavior, effects, dependencies, compatibility, recovery, and evidence; code fills declared roles. Product intent is sufficient input, but semantic changes must be applied before their implementations.

- Treat deterministic RMS output as derived evidence, never as authority to edit source or bypass ownership.
- Use the current project, its rendered RMS context, and its selected skills. Do not borrow semantics from sibling projects, prior runs, RMS source, or generated examples.
- Resolve warnings and `review-required` findings or report them as open obligations.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness.

## Start / Route

1. Run `rms next --task "<intent>" --root .`; add `--module <module.yaml>` only for an explicit owner override.
2. Inspect the prescribed canonical context and declared role paths. For recursive ownership, follow `rms route`; never guess through a tie.
3. Before implementation, use `rms explain`, `rms context`, and the task-selected project skill when prescribed.

For a new or adopted system, the order is:

`rms init [--adopt]` → authorized bootstrap commit → `rms design --root . --task "<intent>"` → recommended scaffold.

- Use `--adopt` when project-owned documents already exist; do not move, overwrite, or restore them around initialization.
- The pre-commit bootstrap state is exactly `bootstrap prepared; provenance baseline pending authorized commit`.
- Choose `rms add-module` for a standalone module or `rms add-capability` for a recommended recursive capability. Supply implementation bindings for work that will produce code.

Provider-backed planning is opt-in. Use `--provider` or `--ai` only when the task explicitly intends an external run; provider output remains advisory until canonical apply succeeds.

## Change Gate

| Requested change | Required declaration before source edits |
| --- | --- |
| Meaning, law, contract, property, artifact, protocol, resource, authority, effect, dependency, evidence | `rms spec plan/apply/check` |
| State, command, event, effect result, transition | `rms spec apply` or focused `rms machine plan/apply/check` |
| App, CLI, UI, HTTP, batch, executable entrypoint | `rms surface apply/check` |
| Module boundary or public capability | `rms design`, then `rms add-module` or `rms add-capability` |
| Deferred implementation binding | `rms add-binding` before machine or surface work |
| Declared role body only | Edit the role body, then run focused verification |

- Dry-run semantic or machine apply first. Use `set` and `remove` operations to revise canonical semantics; never edit an applied revision.
- Replace scaffold contracts and placeholder evidence before production audit.
- After apply, fill only the declared roles and exact symbols. If RMS cannot express the change, stop and report the RMS gap.

## Hard Boundaries

- Do not hand-edit `module.yaml`, `implementation.yaml`, contracts, laws, semantic functions, public/dependency behavior bindings, machine cases, surfaces, protocols, resources, authorities, or evidence declarations.
- Keep public behavior closed through contract → semantic function → classified machine input/output → property or evidence. Close required capabilities through an exact consumer and declared provider or explicit external boundary.
- Keep pure roles free of filesystem, process, network, clock, randomness, persistence, and provider IO. Model boundary IO as declared effects with typed results and dedicated executors.
- Stateful effects follow runnable entrypoint → machine driver → pure transition record → one-request executor → typed result → driver. The driver owns the repeated lifecycle.
- Use closed variants, validated constructors, explicit rejection channels, and checked or bounded arithmetic.
- Import another module only through its declared public facade or contract-shaped entrypoint; never bypass private roles.
- Runnable surfaces adapt inputs and delegate to exact declared callables; they do not duplicate domain decisions or loop around a one-step driver.
- Generate active traces through the real transition-record path. Property runners execute the declared operation and oracle; fixed examples are not generated proof.
- Treat reports, diffs, packages, atlases, prompts, and command logs as evidence, not live project semantics.

## Completion

Use this proof order:

focused native/spec/machine/surface/property/trace/package checks → `rms gate --root .` → authorized candidate commit → `rms audit --root . --strict`.

- Gate must exit zero with no failed check. Resolve or explicitly report every manual obligation.
- If commit authority is absent, stop at exactly `candidate prepared; strict audit pending authorized commit` and do not claim completion or production readiness.
- Strict audit must pass against the clean committed candidate. Record exact checks and remaining obligations in the handoff.

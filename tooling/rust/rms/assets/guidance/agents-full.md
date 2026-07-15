# Agent Instructions

<!-- RMS generated full guidance -->

## Authority

Canonical RMS artifacts own architecture, behavior, effects, dependencies, compatibility, recovery, and evidence; code fills declared roles. Apply semantic changes before their implementations.

- Treat RMS reports as derived evidence, never authority to edit source or bypass ownership.
- Use this project, its rendered context, and its selected skills; resolve warnings and `review-required` findings.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness.

## Start / Route

1. Run `rms next "<intent>" --root .`; use `--module <module.yaml>` only for an explicit owner override.
2. Follow its immediate action. Use `rms explain ["<question>"]` when the reason or canonical meaning is unclear; add `--details` only when the compact answer is insufficient.
3. Inspect prescribed context and role paths, load the selected project skill, and use `rms help --all` only for specialist commands.

New or adopted systems follow `rms init [--adopt]` → authorized bootstrap commit → `rms design --root . --task "<intent>"` → the recommended standalone or recursive scaffold.

- Preserve project-owned documents during adoption.
- Without commit authority, stop at exactly `bootstrap prepared; provenance baseline pending authorized commit`.
- Provider execution is opt-in and advisory until canonical apply succeeds.

## Change Gate

| Change | Declare before source edits |
| --- | --- |
| Meaning, law, contract, effect, dependency, authority, property, evidence | Semantic apply, dry-run first |
| State, input, output, or transition | Semantic apply or focused machine apply |
| App, CLI, UI, HTTP, batch, or executable boundary | Surface apply |
| Module boundary or public capability | Design, then the recommended scaffold |
| Declared role body only | Edit the role, then run focused proof |

- Use `set` and `remove` to revise canonical semantics; never edit an applied revision.
- Fill only declared roles and exact symbols. If RMS cannot express the change, report the RMS gap.
- Replace scaffold contracts and placeholder evidence before production audit.

## Hard Boundaries

- Do not hand-edit canonical manifests, contracts, semantic functions, behavior bindings, machines, surfaces, protocols, authorities, resources, or evidence declarations.
- Keep public behavior closed from contract through semantic function and classified machine input/output to evidence.
- Keep pure roles free of IO; model boundary IO as declared effects with typed results and dedicated executors.
- Let the driver own repeated transition/effect/result lifecycles; surfaces and one-request executors do not loop around it.
- Use closed variants, validated constructors, explicit rejection channels, and checked or bounded arithmetic.
- Cross modules only through declared public facades or contract-shaped entrypoints.
- Generate traces and properties through the declared executable paths, not copied examples.
- Treat reports, diffs, packages, atlases, prompts, and logs as evidence, not live semantics.

## Completion

Use focused native and RMS proof → `rms check --changes --root .` → authorized candidate commit → `rms check --committed --root .`.

- The change check must pass; resolve or report every manual obligation.
- Without commit authority, stop at exactly `candidate prepared; strict audit pending authorized commit`.
- The committed check must pass against the clean candidate. Record exact checks and remaining obligations.

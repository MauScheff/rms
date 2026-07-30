# Agent Instructions

<!-- RMS generated full guidance -->

## RMS Self-Application Boundary

When the task develops, maintains, reviews, documents, or discusses RMS itself—including its CLI, contracts, schemas, guidance, skills, integrations, examples, or release tooling—do not use RMS as the router, workflow, authority, change gate, or completion framework. In particular, do not begin ordinary RMS development with `rms next` or use an RMS-generated route to govern that work. Use the repository's maintainer workflow and native proof instead.

RMS commands may be executed when RMS behavior is the system under test; their output is test evidence, not authority over the work. Use RMS to govern changes to RMS only when the user explicitly requests self-hosting or rewriting the RMS CLI through RMS. This boundary overrides the RMS invocation requirements below for work on RMS itself.

## Authority

Canonical RMS artifacts own architecture, behavior, effects, dependencies, compatibility, recovery, and evidence; code fills declared roles. Apply semantic changes before their implementations.

- Treat RMS reports as derived evidence, never authority to edit source or bypass ownership.
- Use this project, its rendered context, and its selected skills; resolve warnings and `review-required` findings.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness.

## Start / Route

1. Begin work that requests or may require a software change with `rms next "<exact change task>" --root . --ai`; RMS records schema-constrained extraction and routes the validated `rms/intent-model/v0.1`, never provider-proposed topology. Do not invoke RMS for read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, or discussion that requests no change; use native project tools instead. If read-only work reveals a proposed change, stop before editing and run `rms next` with that exact change task. Use typed intent flags only for CI, offline, or intentionally pre-structured caller input, and `--module` only for an explicit caller-owned override. If provider execution fails, do not synthesize typed intent as an automatic fallback. If routing is non-ready or ownerless, do not select or imply an owner from candidates, context, neighboring modules, or implementation language; resolve ownership explicitly, model/adopt the boundary, or state that the work remains outside RMS coverage.
2. Follow its immediate action. Use `rms explain ["<question>"]` when the reason or canonical meaning is unclear; add `--details` only when the compact answer is insufficient.
3. Inspect prescribed context and role paths, load the selected project skill, and use `rms help --all` only for specialist commands.

New or adopted systems follow `rms init [--adopt]` → authorized bootstrap commit → `rms design --root . --task "<exact user task>" --ai` → the exact recommended standalone or recursive scaffold with its `--route-receipt`.

- Preserve project-owned documents during adoption.
- Without commit authority, stop at exactly `bootstrap prepared; provenance baseline pending authorized commit`.
- Provider execution is opt-in and advisory until canonical apply succeeds.

## Change Gate

| Change | Declare before source edits |
| --- | --- |
| Meaning, law, contract, effect, dependency, authority, property, evidence | Semantic apply, dry-run first |
| State, input, output, or transition | Semantic apply or focused machine apply |
| App, CLI, UI, HTTP, batch, or executable boundary | Surface apply |
| Module boundary or topology | Typed design, then `add-module` or `add-capability-tree` exactly as recommended |
| Publish or require a capability on an existing module | Spec apply with `contracts.* kind: capability` plus its behavior binding |
| Declared role body only | Edit the role, then run focused proof |

- Use `set` and `remove` to revise canonical semantics; never edit an applied revision.
- A pure reusable library is an ordinary standalone module. Only a runnable module mixing invariant-bearing decisions with boundary effects needs `unsplit_runnable_justification`.
- Fill only declared roles and exact symbols. If RMS cannot express the change, report the RMS gap.
- Pass the ready route `run_id`, run directory, or receipt file through `--route-receipt` on every prescribed canonical semantic or topology mutation, including dry-runs. Receipts grant neither source-edit nor Git authority.
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

- Temporal promises use typed observations, explicit assumptions, closed expressions, and dimensionally valid quantities. Evaluate real traces; only exhausted finite search supports universal finite conclusions.

## Completion
Use focused native and RMS proof → `rms check --changes --root . [--module <module.yaml>]` → authorized candidate commit → `rms check --committed --root . [--module <module.yaml>]`.

- The change check must pass; resolve or report every manual obligation.
- Without commit authority, stop at exactly `candidate prepared; strict audit pending authorized commit`.
- The committed check must pass against the clean candidate. Record exact checks and remaining obligations.
- In progressive workspaces, state that proof covers discovered RMS module closures only; never imply whole-repository certification.

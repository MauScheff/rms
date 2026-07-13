# Reliable Modular Systems

Reliable Modular Systems is a semantic workbench for coding agents.

RMS lets a builder describe software in natural language, then gives the agent a disciplined path from intent to reliable code. The agent is instructed to clarify ambiguity, surface edge cases, and encode the meaning first: modules, laws, contracts, state machines, effects, evidence, and ownership boundaries. Only after that does it fill the declared roles with implementation code.

The result is software shaped for reliability by default: explicit modules, ADTs or closed variants, traceable state machines, boundary parsers, declared effects, replayable evidence, and deterministic gates that make change reviewable by humans and agents.

RMS owns semantics. Agents fill declared roles. The CLI proves the result.

```text
Model meaning.
Constrain change.
Isolate effects.
Compose through contracts.
Verify the laws that matter.
```

## Why It Exists

Modern codebases fail less from missing abstractions than from unclear ownership. A function signature can say two modules connect; it cannot say whether retries are safe, who owns a state transition, whether an event is a fact or an instruction, or what must remain compatible during replacement.

RMS makes those promises explicit without requiring a framework, language, deployment style, or coding agent. It works for monoliths, libraries, services, workflows, and agent-maintained repositories.

## The Core Loop

```text
user intent
-> RMS semantic structure
-> generated agent guidance
-> declared module and machine roles
-> agent-filled implementation
-> deterministic verification and audit
```

For builders, RMS means you can stay in product language longer. You can say what should happen, what must never happen, and what edge cases worry you. The agent uses RMS to turn that into concrete semantics before writing code.

For developers, RMS means architecture is not hidden in a prompt, a convention, or one agent's taste. The important meaning is committed as canonical artifacts: `module.yaml`, contracts, laws, implementation bindings, transition records, traces, and evidence.

For agents, RMS narrows the job. The agent does not invent architecture directly in source files. It uses the CLI to create or change semantic structure, then edits inside declared roles. Bugs become diagnosable bad states: invalid commands, illegal transitions, unexpected effect results, stale projections, missing evidence, or boundary violations.

## What You Get

- A canonical specification for modules, bounded contexts, contracts, effects, profiles, compatibility, and conformance.
- YAML manifests for systems, context maps, modules, contracts, implementations, semantic changes, machine changes, and conformance reports.
- A semantic gate for changing meaning before code: laws, contracts, commands, states, events, effects, effect results, replies, rejections, transitions, semantic-function authority bindings, public entrypoints, and evidence obligations.
- A semantic revision seal: RMS hash-seals the exact applied change, automatically closes every active prior revision, and recomputes canonical module, contract, and implementation semantics during strict audit.
- Semantic properties for broad laws: RMS declares input spaces, oracles, evidence, replayable counterexamples, and the realization claim. Fixed corpora, finite exhaustive checks, generated properties, and coverage fuzzers remain distinct; every stronger-than-corpus claim names a real binding harness.
- Traceable machine scaffolds with named transition cases, state, effects, transition outputs, journals, replay bundles, and first-bad-transition diagnostics.
- Atomic effect protocols with a trace-complete execution chain: runnable callable -> machine driver -> pure transition record -> exact one-request executor -> typed effect result -> machine driver. Executors are first-class semantic functions; the transition owns what happens next, and the driver retains the live record history for replay and first-bad-transition diagnosis.
- A Rust reference CLI that acts as the human and agent workbench for validation, explanation, context packets, semantic planning, structure checks, trace replay, compatibility, audit, packaging, and conformance evidence.
- Agent skills for inspecting modules, implementing changes, pruning semantic residue, adding modules, evolving contracts, composing modules, and verifying conformance through the shared CLI surface.
- Thin Codex and Claude integration guidance that points agents at the same semantic model instead of creating agent-specific architecture.

## Install The CLI

Requirements:

- Rust 1.89 or newer

For normal use, install a release archive from the GitHub releases page:

```text
https://github.com/reliable-modular-systems/reliable-modular-systems/releases
```

Extract the archive for your platform and put `rms` on `PATH`.

For source installs from a checkout:

```bash
cargo install --path tooling/rust/rms
```

After installation:

```bash
rms config init
rms diagnose
```

Inside a source checkout:

```bash
rms explain "How does this module work?" --root examples/minimal
```

For contributor workflows, run without installing:

```bash
cargo run -p rms -- validate --root examples/minimal
cargo run -p rms -- release check --root .
```

## First Commands

For a guided first pass, use `QUICKSTART.md`. For a self-hosted RMS walkthrough, use `DOGFOOD.md`.

The golden path is:

```text
init -> bootstrap commit -> design -> add-capability with bindings -> spec/surface apply -> implement declared roles -> gate -> candidate commit -> strict audit
```

Create a new RMS system:

```bash
rms init ./my-system \
  --name my-system \
  --purpose "Build reliable modular software" \
  --context core
```

This creates `system.yaml`, `context-map.yaml`, `GLOSSARY.md`, `AGENTS.md`, `.rms/config.yaml`, `.agents/skills/`, and `.gitignore`. Outside an existing worktree it also runs `git init`; inside one it reuses the existing repository. Commit this bootstrap before product work so semantic and source drift have a baseline. The generated agent and workbench files are adapters over the RMS manifests and CLI; they are not a second source of architecture.

Add a module with an implementation binding:

```bash
rms design --root ./my-system \
  --task "browser-playable Snake game"

rms add-module ./my-system/modules/widget \
  --name widget \
  --purpose "Own validated widgets" \
  --kind library \
  --shape domain-engine \
  --binding rust

rms add-module ./my-system/modules/swift-widget \
  --name swift-widget \
  --purpose "Own validated Swift widgets" \
  --kind library \
  --shape domain-engine \
  --binding swift

rms add-module ./my-system/modules/snake-web \
  --name snake-web \
  --purpose "Expose the Snake game as an executable surface" \
  --kind adapter \
  --profile boundary \
  --shape boundary-adapter \
  --binding js
```

Treat deterministic design hints as the scaffold decision. When they recommend a composite/domain/boundary capability tree, use `rms add-capability`; use one module only for explicit library-only intent or a canonically recorded single-module exception.

Add a recursive capability tree when one public capability needs a composite parent plus domain and boundary children:

```bash
rms add-capability ./my-system/modules/tic-tac-toe \
  --name tic-tac-toe \
  --purpose "Expose playable Tic-Tac-Toe" \
  --domain-binding rust \
  --boundary-binding js
```

Choose bindings when the project is expected to produce code. Omitting them creates an intentional semantic-only scaffold. If implementation is deferred, attach it later without copying a scratch scaffold:

```bash
rms add-binding ./my-system/modules/tic-tac-toe-domain/module.yaml --binding rust
rms add-binding ./my-system/modules/tic-tac-toe-boundary/module.yaml --binding js
```

`rms add-binding` preserves the module's laws, contracts, capabilities, and dependencies. It realizes the existing semantic shape through the selected binding adapter, then machine and surface semantics continue through `rms spec apply` or `rms machine apply`.

This creates `module.yaml`, a module `README.md`, `contracts/README.md`, concrete evidence files referenced by the manifests, and optional implementation bindings. Semantic shapes such as `domain-engine`, `boundary-adapter`, `workflow`, `storage-adapter`, `integration-adapter`, and `composite` define role obligations before file layout. Bindings such as `rust`, `swift`, `js`, and `executable` realize those roles idiomatically. The executable binding remains the opaque command-backed lane for web, mobile, CLI, native UI, generated assets, or integration surfaces when RMS cannot statically inspect internals.

Generated inspectable bindings declare inner structure in `implementation.yaml`: representation, binding-native message envelopes, transition output, an exact transition-record function, parser, adapter, journal, timeline projection, replay bundle, first-bad-transition, and trace evidence roles, plus a domain-named machine with an explicit mode and state variants. Effectful drivers return complete transition records rather than output-only histories. They also seed local trace bundles under `verification/traces/`, and `rms verify` checks those bundles after native verification. Role types use a semantic domain prefix, not the module slug: role, binding, and surface suffixes such as `rules`, `engine`, `adapter`, `cli`, `web`, `js`, `rust`, and `swift` are stripped before appending `Machine`, `State`, `Command`, `Event`, `Effect`, `EffectResult`, `Reply`, `Rejection`, `Transition`, and `TransitionRecord`. Prefer the `add-capability` default child paths unless the user supplied better product/domain names; do not invent `-rules`, `-adapter`, `-cli`, or `-web` child names just to describe RMS roles. For example, a `coupon-rules` child under a `coupon-evaluation` capability should expose names like `CouponEvaluationMachine`, not `CouponRulesMachine`, and a boundary role should avoid names like `CouponEvaluationAdapterMachine`.

Semantic properties live above language-specific property-test libraries. A law says what must always hold; a property says which inputs to generate and which oracle judges them; a Rust, JS, Swift, Python, or executable binding decides how to run that property. Revise them through `rms spec apply` with `properties.add/set/remove`. Non-corpus realizations name an exact `path#symbol` harness, and generated evidence remains an obligation until the actual command and observed result are recorded. Inspect and run those obligations with:

```bash
rms property check ./my-system/modules/widget/module.yaml --strict
rms property run ./my-system/modules/widget/implementation.yaml --profile smoke
rms property replay ./my-system/modules/widget/verification/fuzz/counterexamples/failing-case.yaml
```

For app, tool, CLI, local-first reference app, runnable, or smoke-test intents, the boundary child should expose a declared runnable surface. A runnable surface adapts outside input into declared RMS commands, may render or execute declared boundary effects, and must not reimplement domain decisions or call private module internals. Browser, CLI, mobile UI, desktop UI, HTTP route, batch command, and opaque executable are bindings over that same semantic role.

Inspect those declarations with:

```bash
rms structure ./my-system/modules/widget/implementation.yaml
```

Declare a runnable surface before adding real app/UI/CLI files:

```bash
rms surface apply ./my-system/modules/tic-tac-toe-boundary/implementation.yaml \
  --kind runnable-boundary \
  --surface browser \
  --entrypoint public/app.mjs \
  --launch-entrypoint public/index.html \
  --delegates-to src/adapter.mjs#handleBoundaryInput \
  --command tic-tac-toe \
  --effect local-browser-io

rms surface check ./my-system/modules/tic-tac-toe-boundary/implementation.yaml --strict
```

Change product meaning through the semantic gate:

```bash
rms spec plan ./my-system/modules/widget/module.yaml \
  --task "add confirmation before writing a local log"

rms spec apply ./my-system/modules/widget/module.yaml \
  --change-yaml 'spec: rms/semantic-change/v0.1
intent:
  summary: Local log writes require confirmation.
laws:
  add:
    - id: confirmation-before-local-log
      statement: A local log write is requested only after confirmation.
      authority: transition
      enforced_by: transition
contracts:
  add:
    - name: confirm-log
      direction: provided
      version: v1
      command: ConfirmLog
      meaning: Confirm a pending local log draft before requesting persistence.
      accepts: [a pending draft with matching confirmation]
      ensures: [the confirmed draft produces one declared local-write effect]
      rejects: [no pending draft, stale confirmation, malformed draft]
semantic_functions:
  set:
    - id: confirmation-transition
      symbol: src/transition.rs#transition
      kind: transition
      purity: pure
      discharges:
        invariants: [confirmation-before-local-log]
      evidence:
        traces: [verification/traces/confirmation_before_log.yaml]
evidence:
  add:
    - kind: trace
      proves: confirmation-before-local-log
      path: verification/traces/confirmation_before_log.yaml
    - kind: scenario
      proves: confirm-log
      path: verification/scenarios/confirm_log.md'

rms spec check ./my-system/modules/widget/module.yaml
```

Use the focused machine gate when laws, public contracts, and evidence obligations are already correct:

```bash
rms machine plan ./my-system/modules/widget/implementation.yaml \
  --task "add confirmation before writing a local log"

rms machine apply ./my-system/modules/widget/implementation.yaml \
  --change-yaml 'spec: rms/machine-change/v0.1
machine:
  mode: stateful-transition-machine
  states:
    add: [PendingConfirmation]
  commands:
    add: [Confirm]
  replies:
    add: [Confirmed]
transitions:
  add:
    - from: PendingConfirmation
      on: Confirm
      to: Ready
      case: ConfirmPendingDraft
      reply: Confirmed'

rms machine check ./my-system/modules/widget/implementation.yaml
```

`rms spec plan`, `rms machine plan`, and provider output are advisory. Apply first with `--dry-run` and inspect the complete `final_machine` and `final_semantic_functions`; do not write product code while generic scaffold cases remain. Use `semantic_functions.add/set/remove` rather than editing implementation bindings when an exact symbol, authority owner, purity, discharged promise, assumption, or evidence binding changes. Contract changes use `direction: provided` for surfaces owned by the module and `direction: required` for consumer expectations under `requires.capabilities`; `set` and `remove` infer direction only when ownership is unambiguous. Spec apply records and hash-seals the exact change and automatically supersedes every active semantic revision; applied records are append-only. Machine and surface apply also seal their exact records. Strict audit recomputes the canonical revision and record digest, so a clean commit cannot hide direct manifest or change-record surgery. Every transition has a stable `case`; each declared case must exist in the declared transition source, source-only branches are rejected, and every lifecycle state must be reachable from `initial_state`. Replay provenance names that transition source file and exact case, not the evidence YAML itself. Machine apply preserves evidence roles but never generates passing replay evidence from its own declarations; implementation and replay must provide that proof. Generated capability contracts remain incomplete until `contracts.set` supplies product meaning, inputs, outcomes, and rejections without changing their ownership direction.

Runnable app/tool/browser/CLI surfaces stay thin, but their boundary machines still use explicit `state + input -> transition` structure. A boundary command parses or rejects outside input, emitted effects are executed once by declared adapters, and typed effect results return through the same transition. Product lifecycle belongs in a workflow or domain machine rather than hidden in the surface. Browser surfaces normally use `entrypoint: public/app.mjs` for the inspectable controller and `launch_entrypoint: public/index.html` for the host file. Any script loaded by the host file is part of the RMS surface: it should import or call the declared controller/adapter, not duplicate parser, generator, transition, or domain logic in a second browser bundle. Use `--launch-script` when an extra local launch script is intentional and should be checked explicitly.

When behavior depends on external truth, model uncertainty before code: unknown, duplicate, stale, partial, conflicting, delayed, or later-corrected outcomes need explicit recovery, retry, compensation, convergence, or reconciliation evidence.

Validate the included examples:

```bash
rms --version
rms validate --root examples/minimal
rms validate --root examples/commerce
rms validate --root examples/rust
rms validate --root examples/swift
rms validate --root examples/tic-tac-toe
```

Check whether discovered modules compose through declared public requirements:

```bash
rms compose --root .
rms compose --root examples/minimal
rms compose --root examples/tic-tac-toe
```

Route work from a composite parent to the likely owning child module:

```bash
rms route examples/tic-tac-toe/modules/tic-tac-toe/module.yaml \
  --root examples/tic-tac-toe \
  --task "change invalid move rules"
```

`rms context`, `rms plan`, `rms implement`, and `rms review` include the same route evidence automatically when task text targets a composite parent. `rms evidence` uses it to recommend proof lanes such as transition records and replay bundles for domain engines, malformed-input tests for boundary adapters, and parent-export evidence for public behavior changes.

Classify the RMS impact of git changes:

```bash
rms impact
rms impact HEAD~1..HEAD --json
rms gate --dry-run
rms gate HEAD~1..HEAD --json
```

`rms gate` runs affected verification plus a strict semantic and structural preflight. It exits nonzero for missing semantic revisions, invalid machine/effect/trace structure, failed verification, or a missing source revision. It defers only clean-commit worktree checks to the final strict audit; a gate pass is not production proof until the candidate is committed and `rms audit --root . --strict` also passes.

Inspect a module:

```bash
rms inspect examples/commerce/payments.module.yaml
```

Explain a module for a human or agent:

```bash
rms explain examples/commerce/payments.module.yaml
rms explain examples/commerce/payments.module.yaml "What state does this module own?"
rms explain "How does this module work?" --root examples/rust
rms explain --module examples/commerce/payments.module.yaml \
  "How does payment recovery work?" \
  --provider codex
```

Check local transition evidence without a runtime:

```bash
rms trace check verification/traces/transition_trace.yaml
rms trace replay verification/traces/transition_trace.yaml
rms trace diagnose verification/traces/transition_trace.yaml
```

Trace commands inspect JSON or YAML trace bundles with recorded transition records. They reconstruct timelines and identify the first structurally bad transition when the bundle contains enough local evidence; they do not route messages, dispatch effects, or require a runtime framework.

Audit production-readiness blockers:

```bash
rms audit --root .
rms audit --root . --strict
rms audit --root . --strict --include-examples
```

`rms audit` aggregates validation, composition, semantic revision integrity, implementation structure, trace coverage, compatibility, and provenance. Strict audit rejects direct canonical drift, unnamed or unreplayed transition cases, unlinked risk-bearing laws, invalid public domain representation, unresolved runnable delegation, placeholder evidence, and missing trace bundles. Repository-root audits skip illustrative `examples/` modules by default; use `--include-examples` when examples are part of the production claim.

Run repeatable blind-agent dogfood from a clean project root:

```bash
rms dogfood run --scenario checkout-reference --root /tmp/rms-checkout --agent codex
rms dogfood run --scenario nutrition-reference --root /tmp/rms-nutrition --agent codex
```

Dogfood records blind-agent prompts, command logs, generated commits, RMS checks, final strict audit output, elapsed time, and cleanup findings under `.rms/dogfood/`.

Check local RMS and optional AI-provider readiness:

```bash
rms diagnose
rms diagnose --json
rms config init
rms agent diagnose --target codex
rms agent diagnose --target claude
rms agent plugin diagnose --target codex
```

Optional provider and run-record defaults can live in `.rms/config.yaml`:

```yaml
ai:
  default_provider: codex
  codex:
    model: gpt-5-codex
    sandbox: read-only
    # timeout_seconds: 900
    # Use `workspace-write` with `write_scope: module` for module-scoped provider edits.
    # sandbox: workspace-write
    # write_scope: module
runs:
  directory: .rms/runs
```

Provider-backed commands remain explicit. Use `--provider codex` directly, or use `--ai` to select the configured `ai.default_provider`. Codex provider execution supports `--sandbox read-only` and `--sandbox workspace-write`; workspace-write defaults to `--write-scope module`, which runs Codex from the target module directory. Provider execution defaults to a 900 second timeout; set `ai.codex.timeout_seconds` or pass `--provider-timeout-seconds <seconds>` for longer bounded runs. Use `--write-scope root` only when the task intentionally changes system, context, glossary, or cross-module artifacts.

Render advisory workbench prompts. Use `rms intent` as the think-before-code gate when a change needs human intent, accepted rationale, candidate contracts, laws, or proof lanes captured before implementation:

```bash
rms intent examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "understand the desired payment capture behavior before coding"

rms plan examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "add payment capture telemetry"

rms implement examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "add payment capture telemetry"

rms evolve-contract examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "change payment capture failure semantics"

rms evidence examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "prove malformed provider responses are rejected"

rms review examples/commerce/payments.module.yaml \
  --root examples/commerce

rms review examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --impact

rms prompt refactor examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "separate provider mapping from lifecycle decisions"

rms refactor examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "separate provider mapping from lifecycle decisions" \
  --record

rms plan examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "add payment capture telemetry" \
  --record

rms implement examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "add payment capture telemetry" \
  --ai

rms review examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --provider codex

rms explain --module examples/commerce/payments.module.yaml \
  "How does this module work?" \
  --root examples/commerce \
  --ai

rms run list --root examples/commerce
rms run latest --root examples/commerce
rms run inspect <run-id> --root examples/commerce
```

Build a bounded context packet for an agent or reviewer:

```bash
rms context examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "add payment capture telemetry"
```

Generate a local module atlas:

```bash
rms atlas examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --output dist/rms-atlas/payments
```

Emit a conformance report:

```bash
rms conformance examples/minimal/module.yaml \
  --implementation examples/minimal/implementation.yaml
```

Classify manifest compatibility:

```bash
rms check-compat old/module.yaml new/module.yaml
```

Package a module for sharing:

```bash
rms package examples/rust/module.yaml --output dist/rust-example.rms
rms verify-package dist/rust-example.rms
```

`rms package` assembles and verifies the package, records the concrete pass in declared reuse evidence, rebuilds with that proof, and verifies the final artifact. `rms verify-package` remains the independent recheck. Reusable modules are semantic packages first: publish a domain-neutral `provides.capabilities[]` entry with a contract, expose one RMS-declared public facade from `implementation.yaml`, and include package/reuse evidence. Native package files such as `package.json`, `Cargo.toml`, `Package.swift`, or `pyproject.toml` are binding evidence: they describe how to import the facade, not what is reusable.

## Adopt RMS In A Project

Start with one boundary. Do not model every folder. Split when pure invariants, external effects, ownership, replaceability, or evidence needs point to different honest boundaries.

For production-intended projects, use `PRODUCTION.md` as the operating runbook. It defines the required project gate, evidence provenance rules, agent bootstrap flow, and downstream CI template.

1. Treat the repository as a system module.
2. Use `rms design --root . --task "<task>"` when module boundaries or semantic shapes are unclear.
3. Identify one domain boundary with real ownership, invariants, or replaceability pressure.
4. Add `system.yaml`, `context-map.yaml`, and a `module.yaml`.
5. Publish only the contracts other modules may depend on.
6. Declare effects, compatibility, assumptions, and the smallest meaningful verification evidence.
7. Add an `implementation.yaml` that points to native build and verification commands. Use `semantic_functions` for representation constructors, parsers, transitions, adapters, and other symbols that discharge important contracts, invariants, and assumptions.
8. Run `rms validate`, then use `rms context` before implementation work.

When a module should be reused by another module, encode that meaning before code reuse: provider modules declare `provides.capabilities[]`, consumer modules declare `requires.capabilities[]` with the expected contract, and code imports only the provider's RMS public facade or calls a contract-shaped entrypoint. Revise those contracts through `contracts.set` with `direction: provided` or `direction: required`; never publish a consumer dependency merely to make its contract editable. Private role files such as representation, transition, parser, or adapter internals are not consumer surfaces.

Semantic scaffolds are language-agnostic. RMS names roles such as representation, command/event/effect envelopes, transition output, transition records, ports, adapters, journals, timeline projections, replay bundles, composition exports, visibility boundaries, and evidence; each binding chooses idiomatic files or modules. Closed alternatives should use ADTs, sealed variants, enums, or tagged constructors. Values with validity rules should use validated constructors. Lifecycle/order-dependent behavior should expose accepted and rejected transitions that produce stable records and can be replayed to find the first bad transition.

The core profile is always required. Add optional profiles only when they are true:

| Profile | Use when |
|---|---|
| `stateful` | The module owns a lifecycle or consistency boundary. |
| `distributed` | Work crosses process, network, queue, storage, or vendor boundaries. |
| `workflow` | A long-running process coordinates several modules. |
| `boundary` | Untrusted or versioned input enters or leaves the system. |

## Agents

RMS is agent-neutral. Agent instructions are adapters; manifests and contracts remain the architectural source of truth.

For Codex:

- Use `rms init` for new projects; it writes portable `AGENTS.md` guidance, `.rms/config.yaml`, and local `.agents/skills/` from the canonical RMS skills.
- Use `rms agent init --target codex --root .` when adding RMS agent guidance to an existing project without initializing system semantics.
- Use `rms agent sync --target codex --root .` after upgrading the RMS binary; it refreshes generated `AGENTS.md` and `.agents/skills` while preserving existing workbench config.
- Use `rms agent diagnose --target codex --root .` to confirm the project is self-contained for an agent.
- Use `rms agent plugin install --target codex` when you also want the optional user-level Codex plugin installed from the current RMS binary.
- Use `rms agent plugin sync --target codex` after upgrading RMS so Codex reloads the packaged plugin skills.
- Use the plugin wrapper in `integrations/codex/rms` only when installable distribution is useful; it is optional convenience packaging, not a semantic dependency.
- Package skills from canonical `skills/` for plugin releases.
- Make the agent use the shared `rms` CLI: `diagnose`, `design`, `explain`, `route`, `plan`, `implement`, `evolve-contract`, `evidence`, `refactor`, `review`, `prompt`, `run`, `machine`, `trace`, `config`, `context`, `validate`, `compose`, `check-compat`, `verify`, `conformance`, and `audit`.
- Use hooks only to call the shared `rms` CLI.

For Claude Code:

- Use `rms agent init --target claude --root .` to generate `AGENTS.md`, `CLAUDE.md`, `.claude/skills`, and safe workbench defaults.
- Use `rms agent sync --target claude --root .` after upgrading the RMS binary; it refreshes generated `AGENTS.md`, `CLAUDE.md`, and `.claude/skills` while preserving existing workbench config.
- Use the same canonical skills and manifests.
- Treat any Claude-specific plugin as packaging, not semantics.

For any other coding agent, provide a context packet containing the system summary, context map, target module manifest, public contracts, direct dependencies, relevant decisions, and verification commands.

## Repository Map

| Path | Purpose |
|---|---|
| `SPEC.md` | Normative RMS 0.1 pilot specification. |
| `MANIFEST.md` | Manifest model and field reference. |
| `TOOLING.md` | Tooling, packaging, composition, and conformance model. |
| `QUICKSTART.md` | First 10 minutes with the CLI. |
| `PRODUCTION.md` | Production-pilot operating guide, strict audit gate, and CI template reference. |
| `DOGFOOD.md` | Walkthrough using the RMS CLI module itself. |
| `RELEASE.md` | Release process, artifact rules, and done criteria. |
| `GLOSSARY.md` | Canonical terminology. |
| `schemas/` | Draft exchange schemas. |
| `skills/` | Canonical agent skills. |
| `tooling/rust/rms/` | Rust reference CLI. |
| `integrations/codex/rms/` | Codex plugin wrapper. |
| `examples/` | Minimal, commerce, Rust, and Swift example artifacts. |
| `templates/` | Starter docs for modules, contexts, decisions, and glossary entries. |

## Release Readiness

Use the same release gate locally, in CI, and before publishing release artifacts:

```bash
rms release check --root .
```

It runs release metadata checks, RMS CLI tests, canonical artifact validation, `rms-cli` implementation verification, example checks, package creation and verification smokes, release-binary smoke, clean-room PATH install smoke, clean-room recursive dogfood, Cargo packaging, embedded skill asset checks, Codex plugin skill sync, and a temp agent/plugin install-diagnose smoke. It does not invoke optional AI providers.

Use `rms audit --root <project> --strict` before claiming a project is production-ready RMS software. The release gate remains the repository publication gate; strict audit is the project-readiness gate.

For downstream project CI, copy `templates/ci/github-actions-rms-project.yml` and pin `RMS_VERSION` to the reviewed RMS release tag used by the project.

The release process, tag rules, expected artifacts, and done criteria live in `RELEASE.md`.

## Status

This repository is RMS 0.1 Canonical Draft. The semantic core is frozen for pilot use: modules, ownership, contracts, invariants, effects, profiles, composition, substitutability, and conformance.

The Rust CLI is intentionally small but usable. It provides the first enforcement layer: schema validation, semantic reference checks, module inspection and explanation, advisory workbench prompts, optional provider-backed prompt execution, composition checks, inner-structure reports, context packets, compatibility classification, portable package directories, package integrity verification, and conformance reports. Language bindings and deeper static analysis can evolve independently under `tooling/<language>/`.

The CLI is itself an RMS module bundle under `tooling/rust/rms/`: it has a `module.yaml`, published command contracts, an `implementation.yaml`, and evidence paths. This keeps the workbench subject to the same manifest, contract, effect, and verification discipline it asks projects to adopt.

The first implementation binding is Rust. It validates Cargo package shape, crate-root entrypoints, public module declarations, source import roots, public re-exports, explicit external-crate allowlists, primitive type aliases, public domain fields, failure discipline, constructor evidence, query-produced read-model exceptions, Stateful representation declarations, and semantic function source symbols.

Swift is the second binding. It validates Swift package shape, target identity, source entrypoints, import allowlists, public re-exports, primitive type aliases, public stored fields, trap-based failure discipline, constructor evidence, query-produced read-model exceptions, and Stateful representation declarations.

JavaScript scaffolding supports inspectable local bindings for domain engines and boundary adapters, including tagged role constructors, parser/adapter separation, and named `node:test` evidence.

The executable binding is the generic opaque lane. It validates the manifest, declared runnable surface entrypoints, and declared commands, then relies on `commands.build` and `commands.verify` for evidence. RMS does not infer internal domain semantics from executable assets; use it when the implementation surface is web, mobile, CLI, native UI, generated assets, or another project shape without a dedicated static binding.

RMS should not be called 1.0 until it has survived a real reference application, a replacement or migration exercise, and at least one codebase primarily maintained through agents.

## License

Apache-2.0.

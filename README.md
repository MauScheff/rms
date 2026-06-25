# Reliable Modular Systems

Reliable Modular Systems is a small specification and toolchain for building software out of explicit, replaceable modules.

RMS gives each meaningful boundary a manifest: what it owns, what it provides, what it requires, which invariants must hold, which effects it may perform, and what evidence proves the promise. The result is architecture that can be read by humans, bounded for agents, and checked by deterministic tooling.

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

## What You Get

- A canonical specification for modules, bounded contexts, contracts, effects, profiles, compatibility, and conformance.
- YAML manifests for systems, context maps, modules, contracts, implementations, and conformance reports.
- A Rust reference CLI that acts as the human and agent workbench for validation, explanation, context packets, packaging, and conformance evidence.
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

Create a new RMS system:

```bash
rms init ./my-system \
  --name my-system \
  --purpose "Build reliable modular software" \
  --context core
```

This creates `system.yaml`, `context-map.yaml`, `GLOSSARY.md`, `AGENTS.md`, `.rms/config.yaml`, `.agents/skills/`, and `.gitignore`. The generated agent and workbench files are adapters over the RMS manifests and CLI; they are not a second source of architecture.

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

Add a recursive capability tree when one public capability needs a composite parent plus domain and boundary children:

```bash
rms add-capability ./my-system/modules/tic-tac-toe \
  --name tic-tac-toe \
  --purpose "Expose playable Tic-Tac-Toe" \
  --domain-child tic-tac-toe-rules \
  --boundary-child tic-tac-toe-cli \
  --domain-binding rust \
  --boundary-binding js
```

This creates `module.yaml`, a module `README.md`, `contracts/README.md`, and guided verification directories. Semantic shapes such as `domain-engine`, `boundary-adapter`, `workflow`, `storage-adapter`, `integration-adapter`, and `composite` define role obligations before file layout. Bindings such as `rust`, `swift`, `js`, and `executable` realize those roles idiomatically. The executable binding remains the opaque command-backed lane for web, mobile, CLI, native UI, generated assets, or integration surfaces when RMS cannot statically inspect internals.

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

`rms context`, `rms plan`, `rms implement`, and `rms review` include the same route evidence automatically when task text targets a composite parent. `rms evidence` uses it to recommend proof lanes such as transition traces for domain engines, malformed-input tests for boundary adapters, and parent-export evidence for public behavior changes.

Classify the RMS impact of git changes:

```bash
rms impact
rms impact HEAD~1..HEAD --json
rms gate --dry-run
rms gate HEAD~1..HEAD --json
```

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

## Adopt RMS In A Project

Start with one boundary. Do not model every folder. Split when pure invariants, external effects, ownership, replaceability, or evidence needs point to different honest boundaries.

1. Treat the repository as a system module.
2. Use `rms design --root . --task "<task>"` when module boundaries or semantic shapes are unclear.
3. Identify one domain boundary with real ownership, invariants, or replaceability pressure.
4. Add `system.yaml`, `context-map.yaml`, and a `module.yaml`.
5. Publish only the contracts other modules may depend on.
6. Declare effects, compatibility, assumptions, and the smallest meaningful verification evidence.
7. Add an `implementation.yaml` that points to native build and verification commands. Use `semantic_functions` for representation constructors, parsers, transitions, adapters, and other symbols that discharge important contracts, invariants, and assumptions.
8. Run `rms validate`, then use `rms context` before implementation work.

Semantic scaffolds are language-agnostic. RMS names roles such as representation, commands, transitions, ports, adapters, traces, composition exports, visibility boundaries, and evidence; each binding chooses idiomatic files or modules. Closed alternatives should use ADTs, sealed variants, enums, or tagged constructors. Values with validity rules should use validated constructors. Lifecycle/order-dependent behavior should expose accepted and rejected transitions that can be replayed from traces.

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
- Make the agent use the shared `rms` CLI: `diagnose`, `design`, `explain`, `route`, `plan`, `implement`, `evolve-contract`, `evidence`, `refactor`, `review`, `prompt`, `run`, `config`, `context`, `validate`, `compose`, `check-compat`, `verify`, and `conformance`.
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

It runs release metadata checks, RMS CLI tests, canonical artifact validation, `rms-cli` implementation verification, example checks, package creation and verification smokes, release-binary smoke, clean-room PATH install smoke, clean-room recursive dogfood, Cargo packaging, and Codex plugin skill sync. It does not invoke optional AI providers.

The release process, tag rules, expected artifacts, and done criteria live in `RELEASE.md`.

## Status

This repository is RMS 0.1 Canonical Draft. The semantic core is frozen for pilot use: modules, ownership, contracts, invariants, effects, profiles, composition, substitutability, and conformance.

The Rust CLI is intentionally small but usable. It provides the first enforcement layer: schema validation, semantic reference checks, module inspection and explanation, advisory workbench prompts, optional provider-backed prompt execution, composition checks, context packets, compatibility classification, portable package directories, package integrity verification, and conformance reports. Language bindings and deeper static analysis can evolve independently under `tooling/<language>/`.

The CLI is itself an RMS module bundle under `tooling/rust/rms/`: it has a `module.yaml`, published command contracts, an `implementation.yaml`, and evidence paths. This keeps the workbench subject to the same manifest, contract, effect, and verification discipline it asks projects to adopt.

The first implementation binding is Rust. It validates Cargo package shape, crate-root entrypoints, public module declarations, source import roots, public re-exports, explicit external-crate allowlists, primitive type aliases, public domain fields, failure discipline, constructor evidence, query-produced read-model exceptions, Stateful representation declarations, and semantic function source symbols.

Swift is the second binding. It validates Swift package shape, target identity, source entrypoints, import allowlists, public re-exports, primitive type aliases, public stored fields, trap-based failure discipline, constructor evidence, query-produced read-model exceptions, and Stateful representation declarations.

The executable binding is the generic opaque lane. It validates the manifest and declared entrypoint paths, then relies on `commands.build` and `commands.verify` for evidence. RMS does not infer internal domain semantics from executable assets; use it when the implementation surface is web, mobile, CLI, native UI, generated assets, or another project shape without a dedicated static binding.

RMS should not be called 1.0 until it has survived a real reference application, a replacement or migration exercise, and at least one codebase primarily maintained through agents.

## License

Apache-2.0.

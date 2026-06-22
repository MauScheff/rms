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
- YAML manifests for systems, context maps, modules, implementations, and conformance reports.
- Agent skills for inspecting modules, implementing changes, adding modules, evolving contracts, composing modules, and verifying conformance.
- A Rust reference CLI for validation, inspection, context packets, and conformance evidence.
- Thin Codex and Claude integration guidance that adapts the same semantic model instead of creating agent-specific architecture.

## Install The CLI

Requirements:

- Rust 1.89 or newer

From the repository root:

```bash
cargo install --path tooling/rust/rms
```

Or run without installing:

```bash
cargo run -p rms -- validate --root examples/minimal
```

## First Commands

Create a new RMS system:

```bash
rms init ./my-system \
  --name my-system \
  --purpose "Build reliable modular software" \
  --context core
```

Add a module with a language binding:

```bash
rms add-module ./my-system/modules/widget \
  --name widget \
  --purpose "Own validated widgets" \
  --kind library \
  --binding rust

rms add-module ./my-system/modules/swift-widget \
  --name swift-widget \
  --purpose "Own validated Swift widgets" \
  --kind library \
  --binding swift
```

Validate the included examples:

```bash
rms validate --root examples/minimal
rms validate --root examples/commerce
rms validate --root examples/rust
rms validate --root examples/swift
```

Inspect a module:

```bash
rms inspect examples/commerce/payments.module.yaml
```

Build a bounded context packet for an agent or reviewer:

```bash
rms context examples/commerce/payments.module.yaml \
  --root examples/commerce \
  --task "add payment capture telemetry"
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

## Adopt RMS In A Project

Start with one boundary. Do not model every folder.

1. Treat the repository as a system module.
2. Identify one domain boundary with real ownership, invariants, or replaceability pressure.
3. Add `system.yaml`, `context-map.yaml`, and a `module.yaml`.
4. Publish only the contracts other modules may depend on.
5. Declare effects, compatibility, and the smallest meaningful verification evidence.
6. Add an `implementation.yaml` that points to native build and verification commands.
7. Run `rms validate`, then use `rms context` before implementation work.

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

- Keep portable repository guidance in `AGENTS.md`.
- Use the plugin wrapper in `integrations/codex/rms`.
- Package skills from canonical `skills/`.
- Use hooks only to call the shared `rms` CLI.

For Claude Code:

- Keep the minimal adapter in `CLAUDE.md`.
- Use the same canonical skills and manifests.
- Treat any Claude-specific plugin as packaging, not semantics.

For any other coding agent, provide a context packet containing the system summary, context map, target module manifest, public contracts, direct dependencies, relevant decisions, and verification commands.

## Repository Map

| Path | Purpose |
|---|---|
| `SPEC.md` | Normative RMS 0.1 pilot specification. |
| `MANIFEST.md` | Manifest model and field reference. |
| `TOOLING.md` | Tooling, packaging, composition, and conformance model. |
| `GLOSSARY.md` | Canonical terminology. |
| `schemas/` | Draft exchange schemas. |
| `skills/` | Canonical agent skills. |
| `tooling/rust/rms/` | Rust reference CLI. |
| `integrations/codex/rms/` | Codex plugin wrapper. |
| `examples/` | Minimal and commerce example artifacts. |
| `templates/` | Starter docs for modules, contexts, decisions, and glossary entries. |

## Status

This repository is RMS 0.1 Canonical Draft. The semantic core is frozen for pilot use: modules, ownership, contracts, invariants, effects, profiles, composition, substitutability, and conformance.

The Rust CLI is intentionally small but usable. It provides the first enforcement layer: schema validation, semantic reference checks, module inspection, context packets, compatibility classification, and conformance reports. Language bindings and deeper static analysis can evolve independently under `tooling/<language>/`.

The first implementation binding is Rust. It validates Cargo package shape, crate-root entrypoints, public module declarations, source import roots, public re-exports, explicit external-crate allowlists, primitive type aliases, public domain fields, failure discipline, constructor evidence, and Stateful representation declarations.

Swift is the second binding. It validates Swift package shape, target identity, source entrypoints, import allowlists, public re-exports, primitive type aliases, public stored fields, trap-based failure discipline, constructor evidence, and Stateful representation declarations.

RMS should not be called 1.0 until it has survived a real reference application, a replacement or migration exercise, and at least one codebase primarily maintained through agents.

## License

Apache-2.0.

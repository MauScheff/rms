# Reliable Modular Systems

Reliable Modular Systems is a semantic workbench for coding agents.

Describe the product intent. RMS turns it into explicit ownership, contracts, laws, state machines, effects, and evidence before an agent fills the declared implementation roles. The result is reviewable by people and deterministically checkable by tools.

RMS owns semantics. Agents fill declared roles. The CLI proves the result.

For the conceptual tour, read [RMS, Explained](EXPLAINED.md). For a runnable path, use [Quickstart](QUICKSTART.md).

```text
Model meaning.
Constrain change.
Isolate effects.
Compose through contracts.
Verify the laws that matter.
```

## Why It Exists

Software fails when ownership and promises are implicit: who may change state, whether retries are safe, which failures are expected, which effects may occur, and what must remain compatible. RMS records those promises as canonical artifacts instead of leaving them in a prompt, convention, or agent's memory.

The model is language- and runtime-neutral. It supports monoliths, libraries, services, workflows, tools, and agent-maintained repositories without imposing an RMS runtime.

## The Core Loop

```text
product intent
→ canonical RMS semantics
→ declared implementation roles
→ agent-filled code
→ deterministic proof
```

The semantic core covers:

- modules, ownership, public contracts, and dependencies;
- closed commands, events, effects, results, states, and transitions;
- public and resource protocols;
- versioned artifacts and transformations;
- authority boundaries and temporal properties;
- executable traces, properties, packages, and conformance evidence.

Canonical artifacts such as `module.yaml`, contracts, applied revisions, implementation bindings, traces, and evidence remain the source of truth. Reports, prompts, graphs, and agent guidance are derived views.

## Install

Install a platform archive from the [GitHub releases page](https://github.com/reliable-modular-systems/reliable-modular-systems/releases), or install from a source checkout:

```bash
cargo install --path tooling/rust/rms
```

The source build requires Rust 1.89 or newer.

## The Five-Command Surface

```text
rms init [OPTIONS] [PATH]
rms next "<intent>" [--root PATH] [--module MODULE] [--json] [--details]
rms explain ["<question>"] [--root PATH] [--module MODULE] [--json] [--details]
rms check [--environment | --changes | --committed] [--root PATH] [--json] [--details]
rms view [OPTIONS]
```

`rms --help` presents this narrow surface. `rms help --all` reveals specialist commands when a selected skill or detailed report prescribes one.

Each default human response is deliberately small:

```text
Outcome or answer
Why
Next
Done when
```

Use `--details` for owner rankings, complete context, roles, diagnostics, skill origins, and proof evidence. Use `--json` for the versioned `rms.surface/v2` agent envelope. Executable actions contain `program` plus `args`; manual actions carry instructions and authorization. A candidate commit is always manual and `host-required`, never an executable Git prescription.

### Start or adopt a system

```bash
rms init ./my-system \
  --name my-system \
  --purpose "Build reliable modular software" \
  --context core
```

Use `--adopt` when the repository already owns documents that RMS must preserve. Initialization creates the canonical system artifacts, concise agent guidance, local skill copies, safe workbench defaults, and repository hygiene files.

The onboarding order is:

```text
init → authorized bootstrap commit → design → recommended scaffold
```

The bootstrap commit is required provenance only when the task and host policy authorize it. Otherwise stop at `bootstrap prepared; provenance baseline pending authorized commit`.

### Ask what to do

```bash
rms next "add a validated public command" --root ./my-system
```

`next` resolves an owner only from canonical evidence, classifies the prospective task, and gives one ordered prescription without editing files, executing checks, invoking providers, or granting source-edit or Git authority. Ambiguity is reported instead of guessed.

Repository installation, plugin or skill synchronization, and Git status/fetch/commit/rebase/merge/push are classified as `repository-operation` and return `no-rms-change` unless the task actually changes RMS behavior. Existing working-tree diffs do not change that prospective classification.

### Understand the reason

```bash
rms explain --root examples/minimal
rms explain "What does this module own?" --module examples/minimal/module.yaml
```

`explain` answers the question first from canonical evidence. Unsupported questions return `insufficient-evidence` and a deterministic next action. Add `--details` for the full canonical inventory. Provider-backed explanation is an explicit expert workflow discoverable through `rms help --all`; the primary command never invokes a provider.

### Check the right boundary

```bash
rms check --environment --root .  # local readiness
rms check --root .                # canonical validity and composition
rms check --changes --root .      # candidate change gate
rms check --committed --root .    # strict proof of the committed candidate
```

`check` delegates to the existing deterministic engines rather than reimplementing their rules. It exits zero only when its selected checks pass.

Production completion is:

```text
focused proof → check --changes → authorized candidate commit → check --committed
```

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness. Without commit authority, stop at `candidate prepared; strict audit pending authorized commit`.

### Explore the system

```bash
rms view --root . --watch
```

The loopback-only viewer projects modules, contracts, machines, semantic functions, effects, traces, evidence, and source provenance. It is read-only derived evidence, never another semantic source.

## Working With Agents

RMS is agent-neutral. A coding agent needs the CLI, concise repository guidance, and the task-selected skill.

1. Start with `rms next "<intent>" --root .`.
2. Use the compact answer; request `--details` only when needed.
3. Use `rms explain` when canonical meaning is unclear.
4. Load the selected repository skill and follow any prescribed specialist command.
5. Finish through the two `check` completion modes and the host's commit policy.

`rms check --environment` summarizes skill sources detected on disk. Detailed expert diagnostics may report their origins, configuration, digests, and equivalence. Detection does not prove that a host injected a skill into the current task: runtime activation is `unknown`, and precedence is `host-defined`.

Canonical skills live in `skills/`. Generated Codex and Claude copies are distributions, not independent workflow definitions. See [Codex integration](integrations/CODEX.md), [Claude Code integration](integrations/CLAUDE_CODE.md), and [generic agent integration](integrations/GENERIC_AGENT.md).

## Adopt RMS Incrementally

Start with one honest boundary; do not model every folder. Split when language, ownership, pure invariants, external effects, replaceability, or evidence needs justify it.

- Publish only contracts that consumers may depend on.
- Declare effects, failures, compatibility, and evidence before implementation.
- Use closed variants and validated constructors for important domain values.
- Keep lifecycle behavior in explicit accepted and rejected transitions.
- Cross modules only through declared public capabilities or facades.
- Let `next` choose the workflow; detailed mechanics live in the selected skill and rendered RMS context.

For production-intended projects, follow [PRODUCTION.md](PRODUCTION.md).

## Repository Map

| Path | Purpose |
| --- | --- |
| `SPEC.md` | Normative RMS 0.1 pilot specification. |
| `MANIFEST.md` | Manifest model and field reference. |
| `TOOLING.md` | Narrow CLI contract and deterministic tooling model. |
| `QUICKSTART.md` | First 10 minutes with the CLI. |
| `PRODUCTION.md` | Production-pilot operating guide. |
| `DOGFOOD.md` | Self-hosted RMS walkthrough. |
| `RELEASE.md` | Maintainer release process. |
| `GLOSSARY.md` | Canonical terminology. |
| `skills/` | Canonical task-specific agent workflows. |
| `tooling/rust/rms/` | Rust reference CLI and RMS module bundle. |
| `integrations/` | Thin agent adapters and plugin packaging. |
| `examples/` | Minimal and composed example systems. |

## Release Readiness

Repository maintainers use the release workflow in [RELEASE.md](RELEASE.md). Its publication gate verifies metadata, the release binary, clean-room installation, canonical artifacts, examples, packages, embedded skills, generated guidance, and integration distributions without invoking optional providers.

Downstream projects use `rms check --committed --root <project>` before claiming production readiness and should pin the reviewed RMS release used by CI.

## Status

This repository is the RMS 0.1 Canonical Draft. The semantic core is frozen for pilot use; the Rust reference CLI supplies deterministic validation, composition, structure, trace, compatibility, package, agent, and release engines behind the five-command public façade.

RMS should not be called 1.0 until it has survived a real reference application, a replacement or migration exercise, and at least one codebase primarily maintained through agents.

## License

Apache-2.0.

# Reliable Modular Systems

Software can now be changed faster than its meaning can be recovered. A coding agent can produce a plausible patch in minutes while ownership, failure semantics, retry rules, allowed effects, and compatibility promises remain scattered across code, prompts, conventions, and memory. The next change must rediscover—or guess—the architecture.

Reliable Modular Systems (RMS) is a CLI and repository model that makes those decisions explicit before implementation. It records who owns behavior, what the system promises, which states and external effects exist, and what evidence will prove a change. Those artifacts stay with the code, giving people and agents the same durable account of the system.

RMS is useful when work crosses a meaningful boundary: a public API, a stateful workflow, an external effect, a compatibility promise, a reusable module, or a codebase maintained by several people or agents. Start with one honest boundary and expand only when the software demands it. A throwaway script may not need RMS; a system whose promises must survive its current author often does.

The trade is deliberate: when meaning changes, meaning is declared before source code changes. In return, reviewers can see the intended behavior before reading its implementation, agents can fill named roles without inventing a new architecture, and deterministic checks can prove the declared claims. RMS works with existing languages and toolchains and adds no application runtime.

## From Intent to Evidence

```text
product intent
→ explicit ownership and promises
→ declared implementation roles
→ code
→ executable evidence
```

RMS resolves the owner and the required declarations before an agent edits code. Canonical artifacts retain the meaning; agents fill declared roles; the CLI proves the declared claims.

RMS does not replace domain judgment, security review, performance work, incident readiness, or language-specific engineering. It makes the promises those practices depend on visible and checkable.

## Quick Start

Install the current CLI from this checkout with Rust 1.89 or newer:

```bash
cargo install --locked --path tooling/rust/rms
```

Prebuilt archives for tagged versions are published on the [GitHub releases page](https://github.com/MauScheff/rms/releases); the source checkout may be newer than the latest tag.

Then try the read-only workflow in this repository:

```bash
rms check --environment --root .
rms next "inspect the example module" --root examples/minimal
rms explain "What does this module own?" \
  --root examples/minimal \
  --module examples/minimal/module.yaml
rms check --root examples/minimal
```

To begin a new system:

```bash
rms init ./my-system \
  --name my-system \
  --purpose "Process orders reliably"
```

To adopt an existing repository without overwriting project-owned documents:

```bash
rms init --adopt ./existing-system \
  --name existing-system \
  --purpose "Describe the system's durable purpose"
```

Then ask `next` about one real product outcome. In a fresh system, its first useful answer is the design-and-scaffold prescription. Once a module exists, RMS can resolve its owner, surface its promises, prescribe the implementation path, and state exactly what will count as done. The [Quickstart](QUICKSTART.md) walks through the complete change.

## Five Commands

| Command | Purpose |
| --- | --- |
| `rms init` | Establish a new RMS system or adopt an existing repository. |
| `rms next` | Turn an intent into the immediate prescribed action. |
| `rms explain` | Reveal the canonical reason when the prescription is unclear. |
| `rms check` | Check the environment, project, candidate change, or committed proof. |
| `rms view` | Explore the system as a read-only semantic graph. |

The ordinary lifecycle is:

```text
init → authorized bootstrap commit
→ next → explain when needed → follow the prescribed work
→ check --changes → authorized candidate commit → check --committed
```

`next`, `explain`, and `check` render `Outcome or answer → Why → Next → Done when`. Add `--details` for deeper evidence, `--json` for their versioned `rms.surface/v2` agent contract, and use `rms help --all` only when prescribed work requires a specialist command.

## Documentation

| Goal | Read |
| --- | --- |
| Try the complete workflow | [Quickstart](QUICKSTART.md) |
| Understand the model | [RMS, Explained](EXPLAINED.md) |
| Look up commands, results, artifacts, and proof rules | [Reference](REFERENCE.md) |
| Operate RMS in a production-intended project | [Production Pilot Guide](PRODUCTION.md) |
| Understand the CLI and JSON model | [Tooling Model](TOOLING.md) |
| Integrate Codex, Claude Code, or another agent | [Agent Integrations](integrations/README.md) |
| Read the normative model | [Specification](SPEC.md), [Manifest Reference](MANIFEST.md), and [Glossary](GLOSSARY.md) |

## Agents and Authority

An agent starts with `rms next "<intent>" --root .`, consults `rms explain` when needed, and completes through the two `rms check` proof boundaries. The five primary commands are deterministic and provider-free; provider-backed specialist workflows are explicit and opt-in. On-disk skill detection does not prove runtime activation in the current task, because activation and precedence remain host-defined.

Git commits are required evidence, not implied authority. RMS does not grant source-edit, provider, Git, release, or deployment authority. Those decisions remain with the user and host policy.

## Status

RMS 0.1 is a production pilot and canonical draft. The semantic core is frozen for pilot use; the Rust reference CLI implements the five-command façade and the deeper deterministic engines behind it. RMS should not be called 1.0 until it has survived a real reference application, a replacement or migration exercise, and a codebase maintained primarily through agents.

## License

Apache-2.0.

# Reliable Modular Systems

Software can now be changed faster than its meaning can be recovered. A coding agent can produce a plausible patch in minutes while ownership, failure semantics, retry rules, allowed effects, and compatibility promises remain scattered across code, prompts, conventions, and memory. The next change must rediscover—or guess—the architecture.

Reliable Modular Systems (RMS) is a CLI and repository model that makes those decisions explicit before implementation. It records who owns behavior, what the system promises, which states and external effects exist, and what evidence will prove a change. Those artifacts stay with the code, giving people and agents the same durable account of the system.

RMS is useful when work crosses a meaningful boundary: a public API, a stateful workflow, an external effect, a compatibility promise, a reusable module, or a codebase maintained by several people or agents. Start with one honest boundary and expand only when the software demands it. A throwaway script may not need RMS; a system whose promises must survive its current author often does.

The trade is deliberate: when meaning changes, meaning is declared before source code changes. In return, reviewers can see the intended behavior before reading its implementation, agents can fill named roles without inventing a new architecture, and deterministic checks can prove the declared claims. RMS works with existing languages and toolchains and adds no application runtime.

RMS behavioral contracts make those promises executable without language-specific wrappers. `rms/contract/v0.2` clauses observe input, output, before/after state, events, effects, and rejection outcomes; native Rust, Swift, JavaScript, Python, or executable tests emit the same invocation or transition record protocol; one RMS evaluator assigns caller, provider, or binding/evidence blame. Core clauses also support finite exploration and optional cvc5-backed SMT analysis. Production monitoring stays out of process and fail-open.

## From Intent to Evidence

```text
product intent
→ agent extracts typed semantic facts
→ RMS validates facts and chooses topology deterministically
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
rms next "inspect the example module" --root examples/minimal --ai
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

Then give an agent one real product outcome. The agent extracts `rms/intent-model/v0.1` facts without proposing topology; RMS rejects contradictions and material unknowns, then deterministically selects a standalone module or recursive capability tree. Negated intent such as “no runnable interface” remains an explicit absence rather than a flaky keyword signal. The [Quickstart](QUICKSTART.md) walks through the complete change.

## Five Commands

| Command | Purpose |
| --- | --- |
| `rms init` | Establish a new RMS system or adopt an existing repository. |
| `rms next` | Turn an intent into the immediate prescribed action. |
| `rms explain` | Reveal the canonical reason when the prescription is unclear. |
| `rms check` | Check the environment, project, affected delta, or exhaustive release proof. |
| `rms view` | Explore the system as a read-only semantic graph. |

The ordinary lifecycle is:

```text
init → authorized bootstrap commit
→ next → explain when needed → follow the prescribed work
→ check --changes → authorized candidate commit → check --committed
→ rms check --all for exhaustive release or CI certification
```

`check --changes` and `check --committed` select affected RMS owners and actual public-behavior consumers. They distinguish candidate regressions, unchanged baseline debt, native handoffs, and outside-coverage paths. `check --all` retains strict complete-repository certification. `next`, `explain`, and `check` render `Outcome or answer → Why → Next → Done when`. Add `--details` for deeper evidence, `--json` for their versioned `rms.surface/v2` agent contract, and use `rms help --all` only when prescribed work requires a specialist command.

For unattended reliability work, the specialist command `rms hunt --root . --budget 8h` replays old failures, regenerates traces, explores finite models, and orchestrates declared fuzz, sanitizer, and mutation lanes in an isolated checkout. Every behavioral finding must replay. A clean result is explicitly bounded evidence, never a global bug-free claim.

## Documentation

| Goal | Read |
| --- | --- |
| Try the complete workflow | [Quickstart](QUICKSTART.md) |
| Run a replayable bug hunt, then try it on a visual game | [Your First RMS Bug Hunt](FIRST_BUG_HUNT.md) |
| Understand the model | [RMS, Explained](EXPLAINED.md) |
| Understand RMS design and future self-hosting foundations | [Designing RMS for Understandability](UNDERSTANDABILITY.md) |
| Choose purity, generation, composition, and proof-reuse tools | [Functional Core and Composition](FUNCTIONAL_CORE.md) |
| Look up commands, results, artifacts, and proof rules | [Reference](REFERENCE.md) |
| Operate RMS in a production-intended project | [Production Pilot Guide](PRODUCTION.md) |
| Understand the CLI and JSON model | [Tooling Model](TOOLING.md) |
| Integrate Codex, Claude Code, or another agent | [Agent Integrations](integrations/README.md) |
| Read the normative model | [Specification](SPEC.md), [Manifest Reference](MANIFEST.md), and [Glossary](GLOSSARY.md) |

## Agents and Authority

Managed agents begin software-change work with `rms next "<exact change task>" --root . --ai`; they use native project tools for read-only investigation, explanation, review, status or history inspection, ordinary Git/repository/tool operations, and discussion that requests no change. If that work reveals a proposed change, the agent stops before editing and routes the exact change task through RMS. Ordinary CLI callers explicitly opt into provider execution or supply typed intent for CI and offline use. RMS schema-constrains and records extraction, validates the model, chooses topology, issues a route receipt, gates canonical semantic and topology mutations with `--route-receipt`, and proves selected scope deterministically. A standalone module publishes a reusable capability through `rms spec apply`; `rms add-capability-tree` only creates an explicitly selected recursive topology. Adopted repositories use progressive coverage so checks state that they certify RMS module closures, not unrelated code.

Detected skill files are configuration evidence, not proof of runtime activation; the host's injected skill catalog remains authoritative for the current task.

Git commits are required evidence, not implied authority. RMS does not grant source-edit, provider, Git, release, or deployment authority. Those decisions remain with the user and host policy.

## Status

RMS 0.1 is a production pilot and canonical draft. The semantic core is frozen for pilot use; the Rust reference CLI implements the five-command façade and the deeper deterministic engines behind it. RMS should not be called 1.0 until it has survived a real reference application, a replacement or migration exercise, and a codebase maintained primarily through agents.

## License

Apache-2.0.

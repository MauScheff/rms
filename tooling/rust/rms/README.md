# `rms`

`rms` is the first reference CLI for Reliable Modular Systems. It is both the deterministic validator and the shared human/agent workbench for RMS projects.

The CLI is itself described as an RMS module bundle in this directory:

```text
module.yaml
implementation.yaml
contracts/
verification/
```

Install from a release archive when you want the normal user path:

```text
https://github.com/reliable-modular-systems/reliable-modular-systems/releases
```

Install from this repository when working from source:

```bash
cargo install --path tooling/rust/rms
```

Run without installing:

```bash
cargo run -p rms -- validate --root examples/minimal
```

Common commands:

```bash
rms validate --root examples/minimal
rms inspect examples/minimal/module.yaml
rms explain examples/minimal/module.yaml
rms explain examples/minimal/module.yaml "What does this module own?"
rms explain "How does this module work?" --root examples/minimal
rms --version
rms diagnose
rms diagnose --json
rms init ./my-system --name my-system --purpose "Build reliable modular software" --context core
rms next --task "add a validated public command" --root ./my-system
rms next --task "add a validated public command" --root . --module examples/minimal/module.yaml --json
rms config init
rms plan examples/minimal/module.yaml --task "add a public command"
rms implement examples/minimal/module.yaml --task "add a public command"
rms evolve-contract examples/minimal/module.yaml --task "change command failure semantics"
rms evidence examples/minimal/module.yaml --task "prove invalid examples are rejected"
rms refactor examples/minimal/module.yaml --task "separate decisions from effects"
rms review examples/minimal/module.yaml
rms review examples/minimal/module.yaml --impact
rms impact
rms impact HEAD~1..HEAD --json
rms gate --dry-run
rms gate HEAD~1..HEAD --json
rms prompt evidence examples/minimal/module.yaml --task "prove invalid examples are rejected"
rms prompt review examples/minimal/module.yaml --impact
rms plan examples/minimal/module.yaml --task "add a public command" --record
rms implement examples/minimal/module.yaml --task "add a public command" --ai
rms review examples/minimal/module.yaml --provider codex
rms explain --module examples/minimal/module.yaml "How does this module work?" --ai
rms run list
rms run latest
rms run inspect <run-id>
rms validate --root .
rms compose --root .
rms gate --root .
rms audit --root .
rms audit --root . --strict
rms release check --root .
rms context examples/minimal/module.yaml --task "change payment capture behavior"
rms atlas examples/minimal/module.yaml --output dist/rms-atlas/minimal
rms view --root . --watch
rms conformance examples/minimal/module.yaml --implementation examples/minimal/implementation.yaml --strict
```

`rms view` is the experimental system-wide semantic explorer. It serves a loopback-only, read-only cross-layer graph built from modules, contracts, implementation machines, semantic functions, exact public/dependency behavior bindings, traces, evidence, and source provenance. Six focused views cover system topology, behavior paths, machines, proof chains, gap triage, and execution-derived debug timelines. Stable deep links, search, source-backed inspection, and live refresh preserve exact semantic identities. Shape-aware obligations distinguish required gaps and unresolved links from recommendations and non-applicable stages.

Before publishing or sharing this CLI, run:

```bash
rms release check --root .
```

Before claiming an RMS-generated project is production-ready, follow the production-pilot workflow in `../../../PRODUCTION.md` and run:

```text
focused checks → gate → authorized candidate commit → strict audit
```

```bash
rms gate --root <project>
# Authorized manual candidate commit
rms audit --root <project> --strict
```

The quickstart is in `../../../QUICKSTART.md`, the self-hosted walkthrough is in `../../../DOGFOOD.md`, and the release process is in `../../../RELEASE.md` from the repository root.

The onboarding order is `init → authorized bootstrap commit → design → recommended scaffold`.

`rms init` writes the canonical system artifacts plus `AGENTS.md`, `.rms/config.yaml`, `.agents/skills/`, and `.gitignore`. It initializes Git when the target is not already inside a worktree, then prescribes an authorized bootstrap commit before `rms design` and the recommended scaffold. For repositories with existing documents, `rms init --adopt` preflights all collisions, preserves the glossary and project-owned document content, installs idempotent RMS-managed guidance and ignore sections, validates existing RMS manifests, and creates only missing artifacts. It is not an overwrite mode.

`rms next --task "<intent>"` constructs a deterministic, prospective report without editing files, invoking a provider, running checks, or granting source-edit or commit authority. It classifies repository shape, selects a module only from unambiguous canonical evidence, classifies the task lane, renders safely escaped command arguments, and preserves commit actions as manual authorization steps. It exits successfully whenever it can construct a report, including bootstrap, design, and owner-ambiguity states.

`rms diagnose` summarizes skill sources detected on disk; `rms agent diagnose` reports their detailed origins, configuration, digests, and equivalence. Neither command can observe the current thread's injected skill catalog. `runtime_activation` is therefore `unknown` and precedence is `host-defined`; detected does not mean runtime-active.

Git commits are required evidence, not implied authority. This guidance does not grant Git authority. When the task and host policy authorize commits, commit at the prescribed point and run strict audit. Otherwise do not claim RMS completion or production readiness. The pending states are `bootstrap prepared; provenance baseline pending authorized commit` and `candidate prepared; strict audit pending authorized commit`.

`rms add-module` writes `module.yaml`, a module `README.md`, `contracts/README.md`, guided verification directories, and an optional Rust, Swift, or executable binding. The generated guidance routes future work through canonical artifacts without defining module-specific semantics.

The workbench prompt commands are advisory by default. They render bounded, versioned prompts for humans or agents. Use `--record` to write `.rms/runs/<run-id>/request.yaml`, `prompt.md`, and `checks.json`. Use `--provider codex` to execute the prompt through `codex exec`, or `--ai` to use `ai.default_provider` from `.rms/config.yaml`, and record `response.md` plus provider logs. Provider execution is bounded by `ai.codex.timeout_seconds` or `--provider-timeout-seconds`, defaulting to 900 seconds. The CLI remains intentionally conservative and reports missing evidence explicitly instead of claiming more conformance than the artifacts prove.

Optional `.rms/config.yaml`:

```yaml
ai:
  default_provider: codex
  codex:
    model: gpt-5-codex
    sandbox: read-only
    # timeout_seconds: 900
runs:
  directory: .rms/runs
```

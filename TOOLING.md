# RMS Tooling Model

RMS is useful as documentation, but its strongest form combines semantic manifests with deterministic tooling.

> **Prompts explain the architecture. Tooling enforces the architecture.**

## 1. Responsibilities

An RMS toolchain should provide five capabilities:

1. **Discover** modules, contexts, contracts, and bindings.
2. **Validate** manifests, dependency boundaries, and declared effects.
3. **Verify** laws, contracts, scenarios, and boundaries.
4. **Check compatibility** between versions or implementations.
5. **Build context packets** for people and coding agents.

## 2. Recommended neutral command model

The command names below are recommended, not normative:

```text
rms inspect <module>
rms context <module> [--task <task>]
rms route <module> --task <task>
rms init <path>
rms add-module <path>
rms validate [module|contract|implementation]
rms diagnose [system]
rms explain [module] [question]
rms prompt <kind> <module> [--task <task>]
rms plan <module> --task <task>
rms implement <module> --task <task>
rms evolve-contract <module> --task <task>
rms evidence <module> --task <task>
rms refactor <module> --task <task>
rms review <module> [--diff <git-spec>] [--impact]
rms impact [<git-spec>]
rms gate [<git-spec>]
rms atlas <module>
rms run list
rms run latest
rms run inspect <run-id-or-path>
rms config init
rms release check
rms verify [module]
rms check-compat <old> <new>
rms compose [system]
rms package <module>
rms verify-package <package>
rms graph [system|module]
rms conformance [module]
```

A project may expose these through another CLI or build system. The semantic behavior should remain stable.

The current reference implementation lives at `tooling/rust/rms` and implements the first usable subset:

```bash
rms validate --root <path>
rms validate --contract <contract.yaml>
rms init <path> --name <system> --purpose <purpose>
rms add-module <path> --name <module> --purpose <purpose> [--shape domain-engine|boundary-adapter|workflow|storage-adapter|integration-adapter|composite] [--binding rust|swift|js|executable]
rms add-capability <path> --name <capability> --purpose <purpose> [--domain-binding rust|swift|js|executable] [--boundary-binding rust|swift|js|executable]
rms inspect <module.yaml>
rms route <module.yaml> --task "..." [--root <path>] [--json]
rms explain [<module.yaml>] ["question"] [--module <module.yaml>]
rms diagnose [--root <path>] [--json]
rms prompt <kind> <module.yaml> [--task "..."] [--diff <git-spec>] [--impact] [--ai|--provider codex] [--sandbox read-only|workspace-write] [--write-scope module|root] [--provider-timeout-seconds <seconds>]
rms plan <module.yaml> --task "..." [--ai|--provider codex] [--provider-timeout-seconds <seconds>]
rms implement <module.yaml> --task "..." [--ai|--provider codex] [--provider-timeout-seconds <seconds>]
rms evolve-contract <module.yaml> --task "..." [--ai|--provider codex] [--provider-timeout-seconds <seconds>]
rms evidence <module.yaml> --task "..." [--ai|--provider codex] [--provider-timeout-seconds <seconds>]
rms refactor <module.yaml> --task "..." [--ai|--provider codex] [--provider-timeout-seconds <seconds>]
rms review <module.yaml> [--diff <git-spec>] [--impact] [--ai|--provider codex] [--provider-timeout-seconds <seconds>]
rms impact [<git-spec>] [--root <path>] [--json]
rms gate [<git-spec>] [--root <path>] [--dry-run] [--json]
rms atlas <module.yaml> [--root <path>] [--output <directory>] [--force]
rms run list [--root <path>] [--run-root <directory>]
rms run latest [--root <path>] [--run-root <directory>]
rms run inspect <run-id-or-path> [--root <path>] [--run-root <directory>]
rms config init [--root <path>] [--provider codex|none] [--model <model>] [--force]
rms release check [--root <path>] [--skip-cargo-package]
rms context <module.yaml> [--task "..."]
rms compose --root <path>
rms check-compat <old-module.yaml> <new-module.yaml>
rms package <module.yaml> [--output <directory>]
rms verify-package <package-directory>
rms conformance <module.yaml> [--implementation implementation.yaml]
rms verify <implementation.yaml|composite-module.yaml>
```

Other tooling implementations should preserve the same semantic meaning even when implemented in another language.

### `init`

Scaffolds a new RMS system with:

- canonical artifacts: `system.yaml`, `context-map.yaml`, and `GLOSSARY.md`;
- portable agent instructions: `AGENTS.md`;
- local RMS workbench defaults: `.rms/config.yaml`;
- local agent skills: `.agents/skills`;
- repository hygiene defaults: `.gitignore`.

Generated agent and workbench files are operational guidance. They must route future work through RMS manifests, contracts, and the shared CLI; they do not create module semantics. The command refuses to overwrite existing files.

### `add-module`

Scaffolds a valid module directory with `module.yaml`, a module `README.md`, `contracts/README.md`, and guided verification evidence directories. When `--binding rust`, `--binding swift`, or `--binding executable` is supplied, it also creates a minimal `implementation.yaml` and scaffold artifacts that pass that binding's checks. The command refuses to overwrite existing files.

Generated module guidance is an adapter over canonical artifacts. It tells humans and agents how to fill ownership, public contracts, effects, invariants, compatibility, and evidence into `module.yaml`, `contracts/`, `implementation.yaml`, and `verification/`; it does not invent module-specific semantics.

When Stateful, Distributed, Workflow, or Boundary profiles are requested, the scaffold includes the required empty profile section so the manifest validates. Fill those sections with real lifecycle, reconciliation, workflow, or boundary semantics before relying on the profile.

### `add-capability`

Scaffolds a recursive capability tree: a composite parent module at the requested path, a sibling `domain-engine` child, and a sibling `boundary-adapter` child. The parent declares `composition.contains`, exports the boundary child public command, and includes parent scenario evidence so `rms verify <parent/module.yaml>` can roll up composition and child implementation checks.

Use this when `rms design` recommends a composite tree for one public capability. Use `rms add-module` for single modules or when the hierarchy is already in place.

The first language binding is Rust. A Rust implementation binding declares `binding: rust` in `implementation.yaml`; the CLI then checks Cargo manifest shape, package identity, public entrypoint placement, explicit external crate dependencies, source import roots, public external re-exports, declared public modules, primitive type aliases, public domain fields, failure discipline, constructor evidence, query-produced read-model exceptions, Stateful representation declarations, and semantic function source symbols.

The second language binding is Swift. A Swift implementation binding declares `binding: swift` in `implementation.yaml`; the CLI then checks Swift package shape, package and target identity, public entrypoint placement, source imports against `dependencies.allowed_external_modules`, public re-exports, primitive type aliases, public stored fields, trap-based failure discipline, constructor evidence, query-produced read-model exceptions, and Stateful representation declarations.

The generic executable binding declares `binding: executable` in `implementation.yaml`. It is an opaque command-backed binding for implementation surfaces RMS cannot inspect statically yet, including web apps, mobile apps, CLIs, native UI, generated assets, and integration scripts. The CLI validates the implementation manifest and entrypoint paths, records `toolchain.runner`, and relies on `commands.build` and `commands.verify` for proof. It does not infer internal domain semantics from the executable surface.

The first compatibility checker is manifest-level. It classifies public surface removals and contract path changes as breaking, additive public surface changes as compatible additive, and profile/effect/capability/policy changes as requiring operational review.

The first composition checker is manifest-level. It checks required module presence, required capability providers, capability contract compatibility, context-map relationships when both sides are named contexts, externally satisfied capability effects, and direct module dependency cycles.

### `inspect`

Produces a concise view of:

```text
Purpose and ownership
Declared profiles
Public contracts
Direct dependencies
Invariants
Effects and operational semantics
Verification evidence
Compatibility policy
```

### `explain`

Renders an intelligible explanation of a module from canonical artifacts. With an optional question, the command focuses the deterministic answer on ownership, contracts, effects, verification, or compatibility when it can do so without an AI provider. If no module path is supplied, the command infers `module.yaml` from `--root` when exactly one module is available. With `--provider codex`, it renders the bounded `rms.explain@v1` prompt, executes it through Codex, and records the run.

### `diagnose`

Checks local RMS readiness:

```text
CLI version
Expected repository files
Optional `.rms/config.yaml`
Discovered RMS artifact counts
Validation diagnostics
Native tool availability
Optional AI-provider command availability
Run-record directory readiness
Agent workflow guidance
```

Use `rms --version` when only the installed CLI version is needed.

Provider availability is diagnostic only. A missing Codex, Claude, or local-model command must not make deterministic RMS validation fail. Use `--json` for a machine-readable readiness report.

### Workbench config

Optional workbench config lives at `.rms/config.yaml`:

```bash
rms config init
```

```yaml
ai:
  default_provider: codex
  codex:
    model: gpt-5-codex
    sandbox: read-only
    # timeout_seconds: 900
    # Use workspace-write for provider edits, normally constrained to the target module.
    # sandbox: workspace-write
    # write_scope: module
runs:
  directory: .rms/runs
```

Config is operational input only. It can supply provider, model, sandbox, write-scope, provider-timeout, and run-record defaults, but it cannot define RMS module semantics. Provider execution remains explicit: use `--provider codex` directly, or use `--ai` to select `ai.default_provider`.

Codex provider execution supports `--sandbox read-only` and `--sandbox workspace-write`. When workspace-write is selected, RMS defaults `--write-scope module`, runs Codex from the target module directory, and appends provider-scope instructions to the prompt. Provider execution defaults to a 900 second timeout; set `ai.codex.timeout_seconds` or pass `--provider-timeout-seconds <seconds>` for longer bounded runs. Use `--write-scope root` only when the task intentionally changes system, context, glossary, or cross-module artifacts.

### `prompt`

Renders a versioned RMS workbench prompt for a selected workflow:

```text
plan
intent
review
refactor
implement
evolve-contract
prune
evidence
drift
```

The command includes bounded module context, workflow instructions, expected output, deterministic checks, and optional diff context. `--impact` is supported for review prompts and adds a derived RMS impact prelude before the diff. By default it prints the prompt and does not edit files or call an AI provider.

Use `intent` before `plan` or `implement` when the task needs human meaning, accepted rationale, candidate contracts, laws, compatibility, or proof lanes clarified before code changes.

With `--record`, it writes a run record under `.rms/runs`:

```text
request.yaml
prompt.md
checks.json
```

With `--provider codex`, or with `--ai` when `ai.default_provider: codex` is configured, it invokes `codex exec` with the rendered prompt and additionally records:

```text
response.md
provider.stdout.log
provider.stderr.log
```

Provider execution is opt-in. It is an adapter over the rendered prompt, not a new source of RMS semantics.

When provider execution is writable, `request.yaml` records the selected sandbox, write scope, timeout, and execution root. Module write scope is a filesystem constraint for the provider run; the prompt still includes canonical context gathered from the requested project root.

### `run`

Inspects saved workbench run records.

```text
rms run list
rms run latest
rms run inspect <run-id-or-path>
```

`list` summarizes saved runs from `.rms/runs` by default. `latest` inspects the newest generated run id. `inspect` renders request metadata, file presence, validation checks, and response content when present. These commands are read-only over run artifacts.

### `plan`

Shortcut for `rms prompt plan`. Requires `--task` and produces an advisory implementation-planning prompt. Supports `--record`, `--ai`, and `--provider codex`.

### `implement`

Shortcut for `rms prompt implement`. Requires `--task` and produces an advisory implementation prompt. The prompt asks the agent to restate the outcome in owning-context language, classify the change, update public contracts or manifests before code when public meaning changes, preserve module boundaries, choose strong representations, and name focused verification evidence. Supports `--record`, `--ai`, and `--provider codex`. It does not itself edit source files.

### `evolve-contract`

Shortcut for `rms prompt evolve-contract`. Requires `--task` and produces an advisory contract-evolution prompt. The prompt asks for compatibility classification across shape, meaning, failures, authorization, idempotency, ordering, consistency, timeout, retry, stored state, and operations; it also asks for versioning, migration, coexistence, translation, deprecation, and provider or consumer evidence updates. Supports `--record`, `--ai`, and `--provider codex`. It does not itself edit source files.

### `evidence`

Shortcut for `rms prompt evidence`. Requires `--task` and produces an advisory evidence prompt. The prompt asks for the changed promise, the smallest strong evidence, positive and negative cases, and manifest or implementation binding references to update. Supports `--record`, `--ai`, and `--provider codex`. It does not itself edit source files.

### `review`

Shortcut for `rms prompt review`. Includes `git diff` from the requested root by default, or a supplied `--diff <git-spec>`. With `--impact`, the prompt includes a derived RMS impact prelude before the diff. Supports `--record`, `--ai`, and `--provider codex`. The diff and impact prelude are untrusted implementation context, not architecture.

### `impact`

Classifies the RMS semantic impact of the current working tree or a supplied git diff spec. The report maps changed paths to discovered module manifests, contracts, implementation bindings, source roots, verification evidence, operations, glossary files, conformance reports, and workbench config. It recommends deterministic checks such as `rms validate`, `rms compose`, `rms review`, `rms verify`, and `rms check-compat`.

Git paths are evidence about changed files, not semantic authority. Manifest, contract, context, glossary, operation, and implementation-binding changes are therefore reported conservatively as review-required.

### `gate`

Runs the executable RMS checks selected from the same impact analysis:

```bash
rms gate
rms gate HEAD~1..HEAD
rms gate --dry-run --json
```

The gate runs validation for impacted RMS changes, composition for architecture-level changes, and implementation verification for affected modules with implementation bindings. Review prompts, compatibility classification, and missing implementation bindings are reported as manual obligations instead of being silently treated as passed.

`rms gate` depends on git changed-path evidence. In a fresh project that is not a git repository, initialize git first or run the deterministic checks directly: `rms validate --root .`, `rms compose --root .`, and `rms verify <implementation.yaml|composite-module.yaml>`.

### `refactor`

Shortcut for `rms prompt refactor`. Requires `--task` and produces an advisory behavior-preserving refactor prompt. Supports `--record`, `--ai`, and `--provider codex`. Provider execution remains read-only in this advisory lane.

### `context`

Builds a bounded task packet containing only:

```text
System summary and context map
Target module manifest
Applicable glossary entries
Public contracts
Direct dependency contracts
Relevant decisions
Verification commands
```

### `atlas`

Generates a local module atlas under `dist/rms-atlas/<module-name>` by default. The atlas writes `atlas.json` and `index.html` from canonical RMS artifacts, stable node IDs, declared source references, owned concepts, public surface, invariants, effects, state, boundary, compatibility, verification evidence, and deterministic question answers. It is derived evidence, not a new source of architecture. Existing output is preserved unless `--force` is supplied.

### `validate`

Checks:

```text
Embedded JSON Schema validation
Missing or stale references
Duplicate ownership
Undeclared dependencies
Private-boundary violations
Undeclared effects
Invalid profile combinations
Contract and implementation drift
```

### `verify`

Runs the evidence declared by the module rather than assuming a universal test framework.

### `check-compat`

Compares public contracts and operational semantics. It should distinguish:

```text
Compatible additive change
Compatible implementation-only change
Deprecation
Breaking contract change
Breaking state change
Operationally incompatible change
```

### `compose`

Checks whether declared requirements can be satisfied by available providers, including contract versions, operational semantics, service constraints, allowed effects, and forbidden dependency cycles.

For recursive module trees, compose also checks declared containment, child visibility, parent exports, and export backing. A parent export must be published in the parent `provides` section and backed by the named child module's public `provides` entry. Consumers outside a parent boundary cannot depend directly on a child marked `internal`.

Compose also reports advisory semantic shape direction findings. A `domain-engine` depending on boundary, storage, or integration adapters; a `domain-engine` declaring effects or the boundary profile; or a composite parent declaring implementation effects is reported as `review-required`, not as a hard failure in v0.1.

When a repository root contains multiple RMS systems, compose treats the discovered system manifests as one review universe and unions their declared `external_dependencies` before checking required capabilities. A module still needs a matching effect declaration for each external capability it requires.

### `route`

Recommends the owning module for a task when the selected target may be a composite parent or recursive module tree. The report derives candidates from `composition.contains`, `composition.exports`, child semantic shapes, public surfaces, and task language. It is advisory evidence; agents should follow the recommended `rms context` command before editing.

For example, a task about invalid game rules routes toward a `domain-engine` child, while a task about CLI parsing or display routes toward a `boundary-adapter` child.

`rms context`, `rms plan`, `rms implement`, and `rms review` render route evidence automatically when task text targets a composite parent. `rms implement` additionally warns not to add private implementation behavior to the composite parent unless the task changes parent composition, exports, parent contracts, or parent evidence.

`rms evidence` also uses the route recommendation. Domain-engine work receives transition, constructor, property, and accepted/rejected evidence guidance. Boundary-adapter work receives malformed-input, parser-to-domain-command, adapter failure, and contract smoke guidance. Public behavior changes name parent/export contract evidence.

### `package`

Assembles a portable module package directory from the canonical manifest, referenced contracts and evidence, sibling implementation binding when present, generated conformance report, and `PACKAGE.json` metadata with source revision, validator identity, included files, sizes, and SHA-256 checksums. The resulting directory may be archived or used as an input to another registry or artifact system.

### `verify-package`

Verifies a portable package directory before it is trusted by another project, registry, or agent. It checks `PACKAGE.json`, rejects unsafe paths and symlinks, confirms that every declared payload file is present with the expected byte size and SHA-256 digest, rejects undeclared payload files, and validates the included RMS module and conformance artifacts.

### `conformance`

Produces a machine-readable result naming the RMS version, profiles, binding, source revision or artifact digest, validator version, checks, outcomes, and evidence.

Conformance reports include semantic shape direction checks for the evaluated module. Review-required shape findings are emitted as non-failing advisory checks.

## 3. Language-binding interface

A language binding should teach the toolchain how to:

```text
Discover source modules
Identify public exports
Build a dependency graph
Detect boundary violations
Run build, format, and verification commands
Map schemas to language types when desired
Instrument declared effects when supported
Locate generated and private files
```

The binding must not change semantic module meaning.

The executable binding is the fallback when no static language binding applies. It should still declare its public entrypoint, runner, build command, verification command, boundary inputs, observable outputs, and evidence, but its semantic assurance comes from those commands and evidence rather than symbol inspection.

A conceptual binding interface is:

```text
binding.discover(project) -> modules
binding.public_surface(module) -> symbols
binding.dependencies(module) -> edges
binding.effects(module) -> detected effects
binding.commands(module) -> build/verify/format commands
binding.verify(module, category) -> evidence
```

## 4. Deterministic enforcement

Important rules should not rely only on an agent remembering them.

Enforce where possible through:

```text
Static import or package-boundary checks
Schema validation
Contract compatibility checks
Capability permissions
Filesystem and network sandboxing
CI gates
Runtime authorization
Database ownership controls
Message schema registries
Hooks that invoke shared validators
```

Vendor hooks may run checks earlier, but CI should remain the agent-independent authority.

## 5. Agent context and skills

Agent integrations should use the same neutral tooling.

```text
Agent Skill
    -> RMS diagnose/explain/context/validation command
    -> Language binding
    -> Native build and verification tools
```

The CLI is the stable workbench for humans and agents. Skills should make agents invoke the CLI instead of carrying RMS behavior in model-specific prompt text. A skill should not hard-code `npm`, `cargo`, `go test`, `pytest`, or another tool unless it is explicitly a language-binding skill.

Core skills should remain semantic:

```text
inspect-module
implement-change
prune-module
add-module
evolve-contract
compose-modules
verify-module
```

`prune-module` is the semantic-debt lane. It asks whether retained artifacts are reachable from current manifests, contracts, invariants, effects, profiles, compatibility policy, implementation bindings, operational recovery paths, or verification evidence. It should delete, merge, inline, rename, or document residue before new abstractions are added.

Future provider-backed workbench commands should remain orchestration over canonical artifacts, prompt templates, provider adapters, deterministic checks, and run records. They must not redefine RMS semantics.

## 6. CI pipeline

A practical pipeline is:

```text
1. Validate manifests, contracts, and schemas.
2. Check dependency and ownership boundaries.
3. Check public-contract and composition compatibility.
4. Build through the implementation binding with pinned toolchain inputs.
5. Run declared verification evidence.
6. Check semantic residue for unowned helpers, stale fixtures, obsolete generated files, and compatibility shims without consumers or removal conditions.
7. Produce a conformance report tied to the source revision or artifact digest.
8. Generate documentation and graphs; fail if generated artifacts drift.
9. For release artifacts, emit provenance and a dependency inventory when appropriate.
```

Distributed or critical modules may add replay, reconciliation simulation, migration verification, or fault injection.

The RMS repository uses one canonical release gate:

```bash
rms release check --root .
```

The gate runs release metadata checks, formatting, Rust tests, RMS validation, RMS implementation verification, composition and compatibility smokes, package creation and verification smoke, scaffold roundtrip, example binding tests, release-binary smoke, clean-room PATH install smoke, clean-room recursive dogfood, Cargo packaging, and Codex plugin sync validation. It does not invoke optional AI providers. Use `--skip-cargo-package` only for offline local checks.

Release metadata is part of the gate. The Cargo package version, `rms-cli` module version, and packaged Codex plugin version must match. Tag releases are published by `.github/workflows/release.yml`, which builds runner-native CLI archives, packages the Rust source crate, emits SHA-256 checksums, and attaches artifacts to the GitHub release. The operational release runbook lives in `RELEASE.md`.

## 7. Generated artifacts

Tooling may generate:

```text
Dependency and context graphs
Public API documentation
State diagrams
Agent context packets
Contract stubs and clients
Verification scaffolds
Conformance reports
AGENTS.md or vendor-specific summaries
```

Generated artifacts must identify their source and should not be edited as canonical truth.

## 8. Security and agent permissions

A module's declared effects can become an agent-permission model.

An agent working on a pure domain module generally should not need:

```text
Production credentials
Network access
Unrestricted filesystem access
Deployment permissions
Vendor dashboards
```

Tooling should grant the smallest capability set needed for the task. This reduces both accidental damage and prompt-injection exposure.

Repository text, issue descriptions, test fixtures, generated files, and imported documentation should be treated as untrusted data rather than authority. Executable skills, plugins, hooks, MCP servers, and validators should be pinned and reviewed before use. Secrets must remain outside manifests, context packets, logs, and conformance evidence.

For published deployable artifacts, the toolchain should support artifact digests, dependency inventories or SBOMs, and build provenance. These are exchange-layer safeguards rather than domain semantics.

## 9. First reference implementation

The Rust CLI provides schema-backed validation, inspection, context-packet, conformance, and verification commands. The broader milestone remains:

```text
Manifest validator
Module/context inspector
Dependency-boundary checker for primary language bindings
Contract compatibility checker for one schema format
Context-packet generator
Portable package builder
Conformance report
```

A single high-quality reference binding and worked example are more valuable than shallow support for many languages.

The Rust CLI is also an RMS bundle in `tooling/rust/rms/`. Its manifest declares the CLI command surface, filesystem and process effects, published command contracts, Rust implementation binding, and evidence paths. This makes the workbench an example of RMS rather than an exception to it.

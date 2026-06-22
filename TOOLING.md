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
rms context <module> [task]
rms init <path>
rms add-module <path>
rms validate [module]
rms verify [module]
rms check-compat <old> <new>
rms compose [system]
rms package <module>
rms graph [system|module]
rms conformance [module]
```

A project may expose these through another CLI or build system. The semantic behavior should remain stable.

The current reference implementation lives at `tooling/rust/rms` and implements the first usable subset:

```bash
rms validate --root <path>
rms init <path> --name <system> --purpose <purpose>
rms add-module <path> --name <module> --purpose <purpose> [--binding rust|swift]
rms inspect <module.yaml>
rms context <module.yaml> [--task "..."]
rms compose --root <path>
rms check-compat <old-module.yaml> <new-module.yaml>
rms conformance <module.yaml> [--implementation implementation.yaml]
rms verify <implementation.yaml>
```

Other tooling implementations should preserve the same semantic meaning even when implemented in another language.

### `init`

Scaffolds a new RMS system with `system.yaml`, `context-map.yaml`, `GLOSSARY.md`, and `AGENTS.md`. The command refuses to overwrite existing files.

### `add-module`

Scaffolds a valid module directory with `module.yaml`, `contracts/`, and verification evidence directories. When `--binding rust` or `--binding swift` is supplied, it also creates a minimal native library and `implementation.yaml` that pass that binding's checks. The command refuses to overwrite existing files.

The first language binding is Rust. A Rust implementation binding declares `binding: rust` in `implementation.yaml`; the CLI then checks Cargo manifest shape, package identity, public entrypoint placement, explicit external crate dependencies, source import roots, public external re-exports, declared public modules, primitive type aliases, public domain fields, failure discipline, constructor evidence, and Stateful representation declarations.

The second language binding is Swift. A Swift implementation binding declares `binding: swift` in `implementation.yaml`; the CLI then checks Swift package shape, package and target identity, public entrypoint placement, source imports against `dependencies.allowed_external_modules`, public re-exports, primitive type aliases, public stored fields, trap-based failure discipline, constructor evidence, and Stateful representation declarations.

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

### `package`

Assembles a portable module package from the canonical manifest, contracts, conformance requirements, implementation binding, and migration material. The resulting transport may be a directory or an input to another registry or artifact system.

### `conformance`

Produces a machine-readable result naming the RMS version, profiles, binding, source revision or artifact digest, validator version, checks, outcomes, and evidence.

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
    -> RMS context/validation command
    -> Language binding
    -> Native build and verification tools
```

A skill should not hard-code `npm`, `cargo`, `go test`, `pytest`, or another tool unless it is explicitly a language-binding skill.

Core skills should remain semantic:

```text
inspect-module
implement-change
add-module
evolve-contract
compose-modules
verify-module
```

## 6. CI pipeline

A practical pipeline is:

```text
1. Validate manifests and schemas.
2. Check dependency and ownership boundaries.
3. Check public-contract and composition compatibility.
4. Build through the implementation binding with pinned toolchain inputs.
5. Run declared verification evidence.
6. Produce a conformance report tied to the source revision or artifact digest.
7. Generate documentation and graphs; fail if generated artifacts drift.
8. For release artifacts, emit provenance and a dependency inventory when appropriate.
```

Distributed or critical modules may add replay, reconciliation simulation, migration verification, or fault injection.

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

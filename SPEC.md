# Reliable Modular Systems Specification

**Version:** 0.1 Canonical Draft  
**Status:** Normative pilot specification

## 1. Purpose

This specification defines a language-agnostic and agent-agnostic structure for reliable modular software.

It standardizes semantic boundaries, contracts, ownership, effects, operational semantics, compatibility, and conformance. It does not prescribe a programming language, framework, deployment topology, persistence model, or coding agent.

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** indicate requirement levels.

## 2. Design goals

An RMS-compliant system SHOULD make it possible to:

1. understand a module without reading unrelated implementation code;
2. change or replace a module through its public contract;
3. detect undeclared dependencies and effects;
4. reason about retries, concurrency, failure, and recovery;
5. provide agents with bounded, trustworthy context;
6. enforce important rules independently of prompt compliance;
7. implement the same semantic contract in different languages.

## 3. Core concepts

### 3.1 System module

A repository or deployable system MAY be modeled as a system module. A system module defines the public boundary, contained contexts, external dependencies, system-wide invariants, and runtime composition.

### 3.2 Bounded context

A bounded context is a semantic boundary within which one domain model and vocabulary are authoritative. A context MUST own its domain meaning and MUST NOT expose private domain objects as shared mutable state.

### 3.3 Module

A module is a cohesive unit of meaning and ownership with an explicit public contract. Modules MAY be nested. A nested module MUST preserve the public/private boundary of its parent.

### 3.4 Aggregate

An aggregate is a boundary of immediate consistency and invariant enforcement. An aggregate SHOULD be no larger than the state that must change atomically.

### 3.5 Workflow

A workflow coordinates several modules or aggregates over time. It owns coordination state but MUST NOT bypass the invariants or private state of participating modules.

### 3.6 Kernel

The kernel contains small, stable technical primitives shared across modules. The kernel MUST NOT become a repository-wide business model or communication mediator.

### 3.7 Contract

A contract defines a public command, query, event, capability, API, or data exchange. A contract includes semantic and operational behavior when those properties affect consumers.

### 3.8 Effect

An effect is contact with state or reality outside a pure decision boundary, including storage, network calls, time, randomness, messaging, filesystem access, secrets, or external services.

### 3.9 Module package

A module package is a transport-neutral distribution of a public module. It contains the semantic manifest, published contracts, required conformance material, and enough implementation or endpoint information to use or evaluate the module.

### 3.10 Conformance report

A conformance report is machine-readable evidence that a particular system or module implementation was evaluated against a named RMS version, profile set, source revision, implementation binding, and validator version.

### 3.11 Semantic function

A semantic function is an implementation-level function, method, parser, constructor, transition, adapter, or interpreter that carries a named part of a module's declared meaning.

Semantic functions MAY be declared by an implementation binding to connect public contracts, invariants, assumptions, and verification evidence to concrete source symbols. A semantic function declaration MUST NOT create public meaning that is absent from the module manifest or published contracts.

## 4. Core profile requirements

Every declared RMS module MUST satisfy the Core profile. Ordinary private components need not be modeled as separate RMS modules.

### 4.1 Identity and purpose

A module that is public across an ownership boundary, independently replaceable, separately deployable, or claimed as an RMS conformance subject MUST declare:

- a stable name;
- a version;
- a concise purpose;
- its kind or role;
- its owning context or system boundary when applicable.

### 4.2 Language

A bounded context MUST define its authoritative domain terminology. A system glossary MAY index terms across contexts but MUST NOT override context-local meanings.

### 4.3 Ownership

A module MUST declare the concepts, state, data, and business decisions it owns.

A module MUST NOT modify another module's private state directly.

A module MAY request work through a public command or capability and MAY consume information through a public query or event.

### 4.4 Public and private surfaces

A module MUST distinguish its public contracts from its private implementation.

Private implementation includes, unless explicitly published:

- internal tables or storage layouts;
- internal helper functions;
- vendor SDK objects;
- framework-specific objects;
- internal domain events;
- mutable entity instances.

Consumers MUST depend only on published contracts.

### 4.5 Dependencies

A module MUST declare required modules, contracts, and capabilities.

Undeclared dependencies MUST be rejected by conformance tooling when the implementation binding can detect them.

Circular dependencies SHOULD be eliminated. When a semantic cycle is unavoidable, the shared concept SHOULD be extracted or interaction SHOULD be mediated by a contract or workflow.

### 4.6 Invariants

A module MUST declare the important properties that must remain true.

An invariant SHOULD have:

- a stable identifier;
- a plain-language statement;
- an enforcement location;
- verification evidence.

Invariants SHOULD be few, meaningful, and owned by the narrowest valid consistency boundary.

### 4.7 Effects

A module MUST declare externally observable effects and required effect capabilities.

Effects SHOULD be isolated behind explicit ports, capabilities, or interpreters.

Ambient access to global state, network, storage, time, randomness, or secrets SHOULD be minimized and MUST NOT bypass declared permissions.

### 4.8 Contracts

A public contract MUST define shape and domain meaning.

When relevant to consumers, a contract MUST also define:

- preconditions and postconditions;
- failure categories;
- authorization expectations;
- observability or causal identifiers when required by consumers;
- idempotency;
- ordering scope;
- consistency expectations;
- timeout and retry semantics;
- versioning and compatibility policy;
- service constraints when consumers depend on latency, throughput, availability, payload, resource, or cost bounds.

Preconditions and postconditions SHOULD be stable, named, and stated in domain language when they affect consumers. Implementation-local assumptions SHOULD be discharged by types, validated constructors, boundary schemas, state models, or semantic function specifications rather than hidden in comments.

### 4.9 Communication

Modules MAY communicate by direct in-process calls, commands, queries, events, or capabilities.

Communication MUST cross the provider's public contract.

A query MUST NOT perform hidden domain mutation.

An event MUST represent a fact accepted by its owning module. An instruction disguised as an event SHOULD be modeled as a command.

### 4.10 Compatibility

A module MUST declare a compatibility policy for public contracts.

Breaking changes MUST be explicit and versioned. A migration or coexistence path SHOULD be provided for active consumers and owned state.

Implementation details MAY change without a public version change when declared behavior remains compatible.

### 4.11 Verification

A module MUST declare verification evidence in the following categories as applicable:

- **laws:** invariants, state rules, and algebraic properties;
- **contracts:** provider, consumer, schema, and adapter compatibility;
- **scenarios:** meaningful behavior and recovery paths;
- **boundaries:** validation of untrusted or versioned input.

A module MUST NOT be required to use every testing technique. The chosen evidence MUST be proportional to risk and strong enough to demonstrate the declared promise.

### 4.12 Operations

A module MUST declare the operational mechanisms required by its profiles and effects.

When relevant, this includes:

- causal identifiers and observability;
- runtime invariant checks;
- reconciliation and repair;
- migration and rollback procedures;
- runbooks and manual-review paths.

Operational mechanisms MUST use the same public semantics and ownership boundaries as the implementation.

### 4.13 Semantic function bindings

An implementation binding MAY declare semantic functions that map source symbols to module invariants, public contracts, assumptions, and evidence.

Semantic function bindings SHOULD classify the source symbol using the smallest accurate role:

- constructor: raw or internal input to a validated value;
- parser: untrusted or versioned input to a boundary value;
- decision: state and intent to a pure domain decision;
- transition: state and accepted fact to new state;
- projector: fact to read model;
- adapter: external model to or from domain model;
- interpreter: effect request to external result.

For each semantic function, a binding SHOULD identify whether its assumptions are:

- represented by input or output types;
- checked as preconditions;
- preserved as invariants;
- guaranteed as postconditions;
- demonstrated by verification evidence.

Pure semantic functions SHOULD NOT perform undeclared effects. Effectful semantic functions MUST remain within the module's declared effects and required capabilities.

### 4.14 Documentation freshness

The manifest and public contracts MUST be updated when public meaning, ownership, dependencies, effects, or compatibility changes.

Agent instruction files MUST NOT be the sole source of architectural truth.

Canonical semantic artifacts MUST agree. A contradiction among manifests, contracts, invariants, context language, or compatibility declarations is a conformance failure and MUST NOT be resolved by an agent through an undocumented precedence choice.

### 4.15 Trust and secrets

Manifests, public contracts, conformance reports, and agent context packets MUST NOT contain credentials or production secrets.

Executable agent extensions, hooks, plugins, and validation tools SHOULD be version-pinned, reviewable, and granted the least capabilities needed for their task. Their behavior MUST NOT redefine RMS semantic truth.

## 5. Optional profiles

A module MUST declare each additional profile whose conditions apply.

### 5.1 Stateful profile

The Stateful profile applies when a module owns a lifecycle or transactional consistency boundary.

The module MUST additionally declare:

- state model or lifecycle;
- legal transitions;
- consistency and transaction boundary;
- concurrency policy;
- persistence and migration policy.

Illegal transitions MUST be rejected or made unrepresentable.

### 5.2 Distributed profile

The Distributed profile applies when behavior crosses a process, network, durable queue, or external vendor boundary.

The module MUST additionally declare:

- delivery semantics;
- idempotency strategy;
- ordering scope;
- timeout interpretation;
- retry policy;
- duplicate handling;
- partial-failure behavior;
- reconciliation or repair mechanism when external truth can diverge.

When a local state change and message publication must be atomic, the implementation MUST use an outbox, transactional log, or an equivalent mechanism.

At-least-once delivery MUST be paired with idempotent consumption or an equivalent duplicate-control mechanism.

### 5.3 Workflow profile

The Workflow profile applies to long-running coordination across modules or aggregates.

The workflow MUST additionally declare:

- trigger and completion conditions;
- coordination state;
- deadlines and timeouts;
- failure and terminal states;
- compensation or repair paths;
- manual-review states when automatic resolution is unsafe;
- replay or resumption behavior.

A workflow MUST use public contracts and MUST NOT mutate participant-owned state directly.

### 5.4 Boundary profile

The Boundary profile applies where untrusted or versioned data crosses a system or context boundary.

The module MUST additionally declare:

- accepted schema and versions;
- input validation;
- trust and authorization boundary;
- resource limits where relevant;
- malformed-input behavior;
- compatibility and deprecation policy.

Boundary verification SHOULD include fuzzing or generated adversarial cases when parser complexity or security risk justifies it.

## 6. State and event requirements

RMS does not require event sourcing.

A module MAY persist current state, an event log, or both. The choice MUST be consistent with its declared audit, replay, migration, and recovery needs.

A module using events MUST distinguish internal domain events from public integration events when internal evolution would otherwise break consumers.

Published events MUST be versioned or governed by a compatibility policy.

## 7. Substitutability

An implementation MAY claim substitutability for another implementation only when it:

1. provides compatible public contracts;
2. requires compatible capabilities;
3. preserves the declared invariants;
4. provides compatible operational semantics;
5. passes the same required conformance suite;
6. supports required state migration or import/export behavior;
7. satisfies declared service constraints that consumers rely on.

Schema compatibility alone is insufficient for a substitutability claim.

## 8. Agent and language neutrality

### 8.1 Language neutrality

The semantic manifest MUST NOT require a specific programming-language construct.

Language-specific source locations, build commands, package metadata, and static-analysis configuration MUST be placed in an implementation binding rather than the semantic module manifest.

### 8.2 Agent neutrality

A compliant project MUST NOT require a specific coding agent.

Vendor-specific instructions, skills, plugins, hooks, and MCP integrations MAY adapt the system for an agent but MUST NOT alter the semantic meaning of the manifests or contracts.

### 8.3 Prompt versus enforcement

Prompt files MAY guide agent behavior. Deterministic constraints MUST be enforced through tooling, permissions, hooks, CI, runtime checks, or equivalent mechanisms.

### 8.4 Agent-produced changes

Conformance MUST depend on the resulting artifacts and evidence, not on which person or agent produced them. Agent-generated changes SHOULD produce the same reproducible build, verification, compatibility, and provenance evidence required of any other change.

## 9. Manifest and package requirements

A compliant system MUST provide a system manifest or equivalent machine-readable artifact.

Every public module MUST provide a module manifest or equivalent machine-readable artifact.

The canonical exchange format in this draft is YAML. JSON MAY be used when it preserves the same data model.

Manifests MAY contain extension fields prefixed with `x-`. Extensions MUST NOT weaken core requirements or silently change the meaning of standard fields.

### 9.1 Portable module packages

A module package MAY be distributed as a directory, archive, language package, registry artifact, container, or remote-service descriptor. RMS does not prescribe the transport.

A public package claiming RMS compatibility MUST include or reference:

- its module manifest;
- all published contracts;
- required conformance suites or acceptance criteria;
- an implementation binding or endpoint descriptor;
- compatibility and deprecation information;
- migration or import/export behavior when state replacement is claimed.

A module package MAY omit source code. Consumers MUST NOT depend on package contents declared private.

## 10. Conformance

A conformance claim MUST identify:

- the RMS specification version;
- the module or system being evaluated;
- declared profiles;
- implementation binding;
- validator or conformance-suite version;
- source revision or artifact digest;
- result and evidence location;
- individual pass, fail, skipped, and not-applicable outcomes.

Core conformance requires:

1. valid manifests;
2. declared ownership and public contracts;
3. declared dependencies and effects;
4. no detectable private-boundary violations;
5. verification evidence for declared laws and contracts;
6. compatibility policy;
7. all declared profile requirements;
8. consistency among canonical semantic artifacts;
9. no secrets in canonical artifacts or context packets.

A project MAY be partially conformant during migration but MUST label missing requirements explicitly.

## 11. Non-goals

RMS does not mandate:

- microservices;
- event sourcing;
- command/query segregation at deployment level;
- functional programming;
- a particular directory layout;
- repositories or service classes;
- formal verification everywhere;
- one testing technique per category;
- one global domain model;
- one coding agent or plugin platform;
- one package manager, registry, archive, or deployment transport.

## 12. Stability model

The conceptual core of modules, ownership, contracts, invariants, effects, profiles, composition, substitutability, and conformance is frozen for the 0.1 pilot and intended to remain stable.

Manifest syntax, schemas, language bindings, tool commands, and agent integrations MAY evolve on independent version tracks.

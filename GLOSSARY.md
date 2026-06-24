# RMS Glossary

## Adapter

A concrete implementation that translates between a module's port or public contract and a technology, vendor, protocol, or storage mechanism.

## Aggregate

A boundary of immediate consistency around entities and value objects. Changes enter through an aggregate root, which protects the aggregate's invariants.

## Anti-corruption layer

A translation boundary that prevents an external or legacy model from leaking into an internal domain model.

## Bounded context

A semantic boundary within which one domain language and model are authoritative.

## Boundary profile

The RMS profile for untrusted, public, or versioned input and output. It adds validation, trust, resource-limit, and compatibility requirements.


## Canonical artifact set

The manifests, contracts, invariants, context language, compatibility declarations, and active linked decisions that jointly define a module's public meaning. Contradictions within the set are drift, not a precedence choice.

## Capability

A named ability required or provided by a module, expressed independently of a concrete implementation. Examples include a clock, payment gateway, event store, or identity verifier.

## Command

A request for the owner of a capability or state to perform work. A command expresses intent and may be accepted or rejected.

## Compatibility

The degree to which a consumer can continue to use a contract, state representation, or implementation after change.

## Conformance

Evidence that a system or module satisfies a specified RMS version and its declared profiles.

## Contract

A public promise covering shape, meaning, and—when relevant—failure, ordering, consistency, idempotency, authorization, and versioning behavior.


## Conformance report

A machine-readable record of an evaluation against a named RMS version and profile set, tied to a source revision or artifact digest, implementation binding, validator version, outcomes, and evidence.

## Context map

A description of relationships, dependencies, published contracts, and translation boundaries among bounded contexts.

## Core profile

The requirements every RMS module follows: purpose, ownership, public boundary, dependencies, invariants, effects, compatibility, and verification.

## Domain event

A fact accepted inside a bounded context. It may be private to that context.


## Decision record

A concise record of a consequential architectural or domain decision, its context, alternatives, and consequences. It explains intent but does not silently override public contracts or manifests.

## Distributed profile

The RMS profile for work crossing process, network, durable queue, or external-vendor boundaries. It adds delivery, retry, idempotency, ordering, and reconciliation requirements.

## Effect

An interaction with mutable state or the outside world, such as storage, network, time, randomness, messaging, filesystem access, secrets, or external services.

## Entity

A domain object defined by stable identity and continuity over time.

## Event

A fact accepted as having happened. Public integration events are contracts; internal domain events may remain private.

## Idempotent

An operation property under which repeating the same operation with the same identity has no additional accepted effect.

## Implementation binding

A language- and toolchain-specific description of how a semantic RMS module is built, inspected, and verified.

## Integration event

A stable, versioned fact published across a bounded-context or system boundary.

## Invariant

A property that must remain true for a defined state or consistency boundary.

## Intent record

A natural-language artifact that captures the human need, motivating examples, counterexamples, questions, accepted answers, and rejected interpretations before implementation. It is evidence for understanding until accepted semantics are encoded in RMS artifacts.

## Kernel

A small set of stable technical primitives shared by modules. It is not a global business model or communication bus.

## Law verification

Evidence that invariants, state transitions, algebraic properties, or policies hold over the relevant input and state space.

## Module

A cohesive unit of meaning and ownership with an explicit public contract and private implementation.


## Module package

A transport-neutral distribution containing a module manifest, published contracts, conformance material, and implementation or endpoint information sufficient to use or evaluate the module.

## Operational semantics

Behavior that affects composition under real execution, including timeout, retry, duplication, ordering, concurrency, consistency, compensation, and reconciliation.

## Port

An abstract interface through which a module requires or provides an effect or capability.

## Profile

An opt-in RMS requirement set activated by a module's characteristics. Profiles keep the core small while making reliability obligations explicit where needed.


## Provenance

Evidence connecting a generated or released artifact to its source revision, toolchain, dependencies, and validation process.

## Query

A request for information that does not perform hidden domain mutation.

## Rationale

The accepted explanation for why a contract, law, invariant, boundary, compatibility choice, or proof lane has its current shape. Rationale explains public meaning but does not override manifests, contracts, invariants, glossary language, or evidence.

## Reconciliation

A process that compares internal state with external or authoritative reality and records or repairs mismatches.

## Scenario verification

Evidence that important end-to-end or cross-module behaviors, failures, retries, and recovery paths work as intended.


## Service constraint

A measurable operational promise that consumers rely on, such as latency class, throughput, availability, payload limits, resource bounds, or cost characteristics.

## Stateful profile

The RMS profile for modules that own a lifecycle or transactional consistency boundary.

## Substitutability

The ability to replace an implementation while preserving compatible contracts, invariants, required capabilities, operational semantics, and state migration behavior.

## System module

The public modular boundary of a repository or deployable system.

## Ubiquitous language

The precise domain vocabulary used consistently in discussion, documentation, code, contracts, tests, and operations within a bounded context.

## Value object

A domain concept defined by its value rather than identity, normally validated at construction and immutable where practical.

## Workflow

A coordinator of long-running behavior across modules or aggregates. It owns coordination state but not the private state or invariants of participants.

## Workflow profile

The RMS profile for long-running coordination, adding deadlines, terminal states, compensations, resumption, and recovery requirements.

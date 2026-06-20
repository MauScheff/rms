# How to Design Reliable Modular Systems

Reliable software is not software with the most abstractions, tests, or infrastructure. It is software whose structure makes incorrect behavior difficult, important behavior explicit, failures recoverable, and changes verifiable.

A useful summary is:

> **Model meaning, constrain change, isolate effects, compose through contracts, and verify the laws that matter.**

This guide explains a simple, language-agnostic way to do that. It is designed for software written by humans, coding agents, or both.

---

## 1. Reliability begins with structure

An unreliable system often depends on unwritten knowledge:

```text
Do not call this operation twice.
This field may only change after that field.
A timeout may mean the external operation succeeded.
This database table belongs to another team.
This event must be published after the transaction commits.
```

Tests can catch some mistakes, but they cannot replace missing structure. Reliable design turns hidden knowledge into explicit artifacts:

```text
Types describe valid data.
Invariants describe what must remain true.
State models describe legal change over time.
Contracts describe module boundaries.
Effects describe contact with the outside world.
Operational semantics describe retries, ordering, and failure.
Verification demonstrates that the promises hold.
```

The goal is not to eliminate failure. Networks fail, storage fails, vendors disagree, processes crash, and agents make mistakes. The goal is to make failure bounded, visible, explainable, retryable, and repairable.

---

## 2. Think in nested modules

The repository itself can be a module. It has a purpose, a public surface, dependencies, and invariants. Inside it may be bounded contexts and smaller modules.

```text
System module
└── Bounded context
    └── Internal module
        └── Aggregates, values, functions, and adapters
```

The structure is recursive, but the terms are not interchangeable.

- A **system module** is the public boundary of a repository or deployable system.
- A **bounded context** is a semantic boundary within which one domain language and model are valid.
- An **internal module** is a cohesive implementation unit inside a context.
- An **aggregate** is a boundary of immediate consistency and invariant enforcement.
- A **workflow** coordinates work across aggregates or contexts without owning their private rules.

A repository may contain one module, one bounded context, or many contexts. Do not introduce more levels than the domain requires.

---

## 3. Start with language and ownership

Domain-driven design contributes two foundational ideas: **ubiquitous language** and **bounded contexts**.

### Use precise domain language

The same important terms should appear in product discussion, code, contracts, events, tests, logs, and documentation.

Prefer:

```text
AuthorizePayment
PaymentCaptured
InventoryReservation
RefundEligibility
```

Avoid:

```text
process
handle
updateStatus
doAction
syncRecord
```

Precise names reduce the amount of hidden context an agent or person must infer.

### Allow local meanings

A term may mean different things in different contexts. `Customer` might mean a buyer in Ordering, a legal account in Billing, and an authenticated principal in Identity. Forcing one universal object usually creates a vague model full of optional fields.

The recommended documentation model is:

```text
GLOSSARY.md
    A system-wide index of terms and their owning contexts.

contexts/<context>/README.md
    The authoritative local language and model for that context.

contexts/<context>/GLOSSARY.md
    Optional when the local vocabulary becomes substantial.
```

The root glossary points to local definitions; it does not erase contextual meaning.

### Give every rule one owner

Every important piece of state, invariant, and business decision needs a clear owner. Other modules may request work or consume facts, but they should not mutate the owner's private state.

Ownership is the foundation of modularity:

> **A module may ask another module to act. It may not act on the other module's behalf.**

---

## 4. The canonical module model

A reliable module is not merely a folder. It is a semantic unit with an explicit contract.

```text
Module
= Purpose
+ Language
+ Ownership
+ Public contracts
+ Required capabilities
+ Invariants
+ Effects
+ Operational semantics
+ Verification evidence
```

Not every internal folder needs a manifest. A module needs a public semantic contract when it is consumed across an ownership boundary, independently replaceable, separately deployable, or important enough that its invariants and effects must be inspected without reading implementation code.

A module should answer:

```text
Why does it exist?
What concepts and data does it own?
What commands, queries, events, or capabilities does it provide?
What does it require from other modules or the runtime?
What must never become false?
What state changes are legal?
What external effects can occur?
What happens on timeout, retry, duplication, reordering, or partial failure?
How is compatibility demonstrated?
```

The implementation remains private. Other modules should not depend on its tables, internal helpers, framework objects, or vendor SDKs.

---

## 5. Contracts are more than schemas

A function signature or message schema describes shape. A reliable contract also describes meaning.

A complete contract may include:

```text
Input and output shape
Preconditions and postconditions
Domain meaning
Failure categories
Idempotency behavior
Ordering requirements
Consistency expectations
Timeout semantics
Version compatibility
Authorization requirements
Observability identifiers
```

This matters for replaceability. Two implementations can share a type signature while behaving differently under retry or failure.

A module is genuinely substitutable only when the replacement:

1. provides compatible public contracts;
2. requires the same capabilities or a compatible subset;
3. preserves the declared invariants;
4. has compatible operational semantics;
5. passes the same conformance suite;
6. provides a migration path for owned state when necessary.

That is the difference between visual “Lego blocks” and real semantic composability.

A contract should also declare service constraints when consumers rely on them, such as maximum payload size, latency class, throughput, availability, resource bounds, or cost characteristics. These are not universal requirements, but a replacement is not operationally compatible when it violates a promise the consumer depends on.

### Portable module packages

For distribution, a module can be published as a transport-neutral package containing:

```text
module.yaml
public contracts
required conformance suite
implementation binding or service descriptor
compatibility and migration information
optional source, binaries, adapters, and documentation
```

The package may be a directory, archive, registry artifact, library, container, or remote service descriptor. RMS standardizes the semantic contents, not the transport.

A package composes safely when:

```text
Its provided capabilities satisfy declared requirements.
Its contract versions and meanings are compatible.
Its operational semantics and service constraints fit the consumer.
Its effects are allowed by the host system.
Its conformance suite passes.
Owned state can be migrated when replacement requires it.
```

This makes “plug and play” a checkable claim rather than a metaphor.

---

## 6. Keep the kernel small

The kernel is shared vocabulary, not a communication bus and not a global business model.

A healthy technical kernel may contain:

```text
Result and optional-value abstractions
Stable identifiers
Time and duration primitives
Validation primitives
Money or quantity primitives when truly universal
Command and event envelopes
Correlation and causation identifiers
Schema-version primitives
```

It should not contain general business objects such as `Customer`, `Order`, `Payment`, or `Product` unless two contexts explicitly and intentionally share a domain kernel.

The kernel should be boring, stable, and dependency-light. Modules communicate through contracts; they do not communicate “through” the kernel.

---

## 7. Model data and change explicitly

### Values and identities

Use value objects for concepts defined by value:

```text
Money
EmailAddress
DateRange
Percentage
Coordinates
```

A value object should be valid when created, immutable where practical, and compared by value.

Use entities for concepts with identity and continuity over time:

```text
Order
Payment
Subscription
Shipment
```

Prefer domain-specific identifiers over interchangeable strings or integers.

### Make illegal states difficult to represent

Use the strongest representation available in the implementation language:

- algebraic data types or sealed variants;
- validated constructors;
- schemas at runtime boundaries;
- opaque or branded identifiers;
- explicit result types for expected failure.

Dynamic languages can achieve the same semantic goal with constructors, validators, schemas, and disciplined module boundaries. RMS requires the property, not a particular type-system feature.

### Use state machines where time matters

Not every record needs a state machine. Use one when legal behavior depends on lifecycle or order.

```text
NotStarted -> Authorized -> Captured -> Refunded
```

A state model should make illegal transitions explicit and testable. It may be represented as code, a transition table, or a declarative specification.

### Keep consistency boundaries small

An aggregate is the boundary of immediate transactional consistency. Put state together only when its invariants must be enforced atomically. Connect the rest by identifiers, queries, commands, events, and workflows.

Large object graphs and cross-module transactions create contention and coupling. Strong consistency should be deliberate, not accidental.

---

## 8. Separate decisions from effects

Pure logic answers what should happen. Effects make it happen in the outside world.

A useful conceptual shape is:

```text
Decide: current state + intent -> decision
Apply: current state + fact -> new state
Interpret: effect request -> external result
Project: fact -> read model
```

The exact functions are optional. The separation is the important part.

Effects include:

```text
Persistent storage
Network calls
Clocks and randomness
Message publication
Email and notifications
Payments and shipments
Filesystem access
Secrets and configuration
```

A module should receive effects through explicit ports or capabilities rather than acquiring ambient power from global state.

This produces several benefits:

- domain decisions can be verified without infrastructure;
- production, test, simulation, and replay interpreters can share the same core;
- permissions can be limited to declared capabilities;
- hidden dependencies become visible.

---

## 9. Declare operational semantics

Distributed failure is often ambiguity rather than a clean error. A timeout can mean “nothing happened,” “it happened but the response was lost,” or “the outcome is still unknown.”

For each important effect or operation, declare what matters:

```text
Idempotency: Can it be repeated safely, and under which key?
Ordering: Does order matter, and what is the ordering scope?
Concurrency: Can operations run in parallel?
Consistency: What is immediately consistent and what converges later?
Timeout: Does timeout imply failure or unknown outcome?
Retry: Which failures are retryable, with what policy?
Compensation: Can an accepted result be offset or repaired?
Reconciliation: How is internal state compared with external truth?
```

Useful algebraic properties should be stated when they provide leverage:

- **Idempotent:** repeating an operation has no additional effect.
- **Commutative:** order does not change the result.
- **Associative:** grouping does not change the result.
- **Monotonic:** knowledge or progress only moves forward in a defined order.
- **Compensatable:** a later action can restore an acceptable state.

Do not annotate everything for mathematical elegance. Declare properties that affect composition, retry, parallelism, or recovery.

---

## 10. Modules communicate through public meaning

Inside one module, direct function calls are normal. Between modules, communication should cross a public contract.

The common forms are:

- **Command:** a request for an owner to perform work.
- **Query:** a request for information without hidden mutation.
- **Event:** a fact the owner has accepted as true.
- **Capability:** an abstract service required or provided by a module.

Direct in-process calls are fine when they use the public contract and preserve ownership. Asynchronous events are useful when decoupling, auditability, replay, or independent timing matters. Neither style is universally superior.

### Domain events and integration events

A domain event may be internal to a context and use its private language. An integration event is a stable, versioned message published to other contexts.

A context should be able to refactor its internal model without breaking every consumer. Translate internal events into public integration events when necessary.

### Anti-corruption layers

External and legacy systems have their own models. Translate them at the boundary rather than letting their terminology and assumptions spread through the domain.

### Workflows coordinate; they do not own everything

A workflow reacts to public outcomes and issues commands to module owners. It owns coordination state, deadlines, and compensations, but not the private invariants of the participating modules.

---

## 11. Use profiles to keep the method proportional

The Core profile applies to every module. Other profiles are opt-in.

### Core profile

Use for every module. It requires:

```text
Purpose and ownership
Public and private boundary
Declared dependencies
Explicit invariants
Declared effects
Compatibility policy
Verification evidence
```

### Stateful profile

Use when a module owns a lifecycle or consistency boundary. Add:

```text
State model
Legal transitions
Transactional boundary
Concurrency policy
Persistence and migration policy
```

### Distributed profile

Use when behavior crosses a process, network, durable queue, or external vendor boundary. Add:

```text
Delivery semantics
Idempotency
Ordering scope
Timeout and retry semantics
Duplicate handling
Outbox/inbox or equivalent atomicity mechanism when needed
Reconciliation and repair
```

### Workflow profile

Use for long-running coordination across modules. Add:

```text
Workflow state
Trigger and completion conditions
Deadlines
Failure states
Compensations
Manual-review paths
Recovery and replay behavior
```

### Boundary profile

Use where untrusted or versioned data crosses the system boundary. Add:

```text
Input validation
Schema and compatibility policy
Authorization and trust boundary
Resource limits
Boundary and fuzz verification where risk justifies it
```

A module may declare several profiles. The profiles say which reliability questions must be answered; they do not prescribe a framework.

---

## 12. Verify the promises, not every implementation detail

A compact verification model is enough:

```text
Laws
Contracts
Scenarios
Boundaries
```

### Law verification

Checks what must always remain true:

```text
Aggregate invariants
State-transition legality
Idempotency
Serialization round trips
Monotonicity or merge laws
Authorization policies
```

Property-based testing, generated state-machine exploration, model checking, or ordinary examples are techniques. Use the smallest technique that strongly demonstrates the law.

### Contract verification

Checks module and adapter boundaries:

```text
Schema compatibility
Published behavior
Consumer/provider agreement
Adapter conformance
Backward compatibility
Anti-corruption translations
```

### Scenario verification

Checks meaningful flows and recovery:

```text
Happy path
Important rejection path
Timeout and retry
Duplicate delivery
Partial failure
Compensation
Historical replay when relevant
```

### Boundary verification

Checks untrusted input and parsers:

```text
Malformed data
Extreme sizes
Unexpected fields or enum values
Invalid encoding
Resource exhaustion
Adversarial sequences
```

Fuzzing belongs here when the boundary or risk justifies it. It is not a universal test category for every internal function.

### Production verification

Tests are not the final authority for external reality. Important distributed modules should also have:

- runtime invariant checks;
- reconciliation against external systems;
- causal observability;
- repair procedures and runbooks.

---

## 13. Design for agents without depending on agents

Coding-agent instructions should help an agent discover and follow the architecture, but they must not be the only place where the architecture exists.

The canonical semantic artifacts should remain neutral:

```text
system.yaml
module.yaml
contracts/
invariants/
context maps
glossaries
verification declarations
decision records where needed
```

These artifacts form one coherent set. If they contradict one another, that is architectural drift and should fail validation; an agent should not silently choose whichever file appears first.

Agent-specific files adapt that truth:

```text
AGENTS.md
CLAUDE.md
Agent Skills
Plugins
Hooks
MCP integrations
```

The rule is:

> **Prompts explain the rules. Deterministic tooling enforces them.**

### Treat agent output as a proposed change

Agent-written code should not receive weaker scrutiny than human-written code, nor should reliability depend on the identity of the model that produced it. A change becomes trusted only after reproducible checks establish that it preserves the module contract.

Use least privilege:

```text
Do not place secrets in manifests or context packets.
Do not grant production credentials for ordinary development.
Pin and review executable skills, plugins, hooks, and MCP servers.
Treat repository text, issues, fixtures, and generated content as potentially untrusted input.
Record the source revision, validator, binding, and evidence used for conformance.
```

An agent may ignore or misunderstand an instruction. A dependency checker, contract validator, permission boundary, or CI check should still reject an invalid change.

### Give agents a small context packet

For a task, an agent usually needs:

```text
System purpose and context map
Target module manifest
Applicable glossary entries
Public contracts
Direct dependency contracts
Relevant decisions
Verification commands
```

It usually does not need the entire repository. Small, explicit context improves both reliability and cost.

### Portable skills

Repeated workflows should be packaged as vendor-neutral Agent Skills:

```text
inspect-module
implement-change
add-module
evolve-contract
compose-modules
verify-module
```

The skill describes the semantic workflow. Language bindings and repository tooling supply the concrete commands.

---

## 14. Remain language- and agent-agnostic

RMS separates three layers:

```text
Semantic core
    Modules, contracts, invariants, effects, profiles, conformance

Language bindings
    How those concepts map to a particular language and toolchain

Agent bindings
    How Codex, Claude Code, or another agent discovers and follows them
```

The semantic contract must not depend on classes, traits, interfaces, decorators, macros, or package-manager conventions. A language binding may explain how to implement the contract in that language, but it may not change the contract's meaning.

Similarly, `AGENTS.md` and `CLAUDE.md` are integration surfaces, not architectural authorities.

---

## 15. A practical repository structure

The directory layout is recommended, not normative:

```text
/
├── README.md
├── system.yaml
├── GLOSSARY.md
├── context-map.yaml
│
├── kernel/
├── contexts/
│   ├── ordering/
│   ├── payments/
│   └── fulfillment/
│
├── workflows/
├── interfaces/
├── runtime/
├── verification/
├── ops/
│
├── AGENTS.md
├── CLAUDE.md
└── skills/
```

A bounded context may use:

```text
contexts/payments/
├── README.md
├── module.yaml
├── domain/
├── application/
├── ports/
├── adapters/
├── contracts/
└── verification/
```

A smaller project may flatten this structure. What matters is that ownership, public surfaces, and dependency direction remain explicit.

---

## 16. Common overcorrections

RMS deliberately does **not** require:

```text
A microservice per module
Event sourcing everywhere
A state machine for every record
A repository class for every entity
All possible testing techniques
A global enterprise domain model
A universal shared kernel
Indirect communication for every local call
Vendor-specific agent configuration in the core spec
```

Use complexity in proportion to uncertainty, business importance, and failure cost.

---

## 17. The final checklist

A reliable module should make these answers easy to find:

```text
Meaning: What does this module mean?
Ownership: What state and decisions does it own?
Boundary: What is public and what is private?
Contracts: What does it provide and require?
Invariants: What must never be false?
State: Which changes are legal?
Effects: What contact with reality can occur?
Semantics: What happens under retry, timeout, duplication, and concurrency?
Compatibility: What can change without breaking consumers?
Verification: What evidence supports the promises?
Composition: What can satisfy this module’s requirements?
Packaging: What must travel with a replaceable implementation?
Trust: Which tools and permissions may act on this module?
Recovery: How are mismatches detected and repaired?
```

The entire method can be compressed to one sentence:

> **A reliable system is a hierarchy of modules with precise language, local ownership, explicit contracts, constrained state change, declared effects, compatible operational semantics, and focused verification.**

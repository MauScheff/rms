# RMS, Explained

AI can produce a working demo in minutes. It can also produce **AI slop**: plausible-looking code with arbitrary boundaries, duplicated business rules, stringly states, hidden side effects, and tests that prove fixtures rather than behavior. The software may work today while becoming difficult for the next agent to understand, diagnose, or change safely.

RMS is designed to prevent that failure mode.

Reliable Modular Systems (RMS) is a CLI that coding agents use to turn natural-language product intent into constrained, inspectable, and verifiable software. It makes an agent define meaning and structure before filling in code, then executes the evidence needed to prove that the implementation matches those claims.

In short:

```text
AI without RMS: intent -> plausible code -> architectural drift

AI with RMS:    intent -> explicit semantics -> declared structure
                       -> implementation -> executable proof
```

The intended user experience is simple:

```text
Describe the product
        |
        v
Agent uses RMS
        |
        v
Working, structured, verified software
```

Most RMS commands are an API for the coding agent. The user should primarily describe what the software should do, clarify product decisions when necessary, and review the result. RMS carries the architectural discipline so every agent does not invent a new structure on every turn.

## The Core Rule

```text
RMS owns semantics and architecture.
Agents fill declared roles.
The CLI proves the result.
```

RMS has three jobs:

1. Define what the software means.
2. Give the implementation a reliable shape.
3. Prove that the implementation matches its claims.

## 1. Define Meaning Before Code

Before implementation, RMS records the important product semantics:

- what the system promises;
- what must never happen;
- which inputs are accepted or rejected;
- which states and outcomes exist;
- which effects interact with the outside world;
- which public contracts connect modules;
- what evidence will prove each important promise.

These become canonical laws, contracts, machine variants, transitions, effects, and evidence obligations. The coding agent may propose them, but they become authoritative only after RMS applies and validates them.

This makes product meaning visible outside the implementation. A behavior change that exists only in source code is semantic drift.

## 2. Shape the Software

RMS structures software at two levels: modules on the outside and machines on the inside.

### Modules: Reusable Semantic Blocks

A module owns one coherent capability and exposes a public contract. A product may be a recursive composition of modules:

```text
checkout
|-- checkout-domain       pure decisions
|-- checkout-workflow     lifecycle and coordination
`-- checkout-boundary     CLI, UI, files, or external providers
```

Modules communicate through declared capabilities and contracts. A consumer cannot rely on another module's private representation, transition, parser, or adapter files.

This is what makes an RMS module reusable: RMS defines what the block means and promises; a language package defines how code imports it.

### Machines: Explicit Behavior Inside Modules

Every implemented module exposes a domain-named machine. Its canonical form is:

```text
State + Input -> Transition
```

Inputs are classified:

- **Command:** asks a machine to do something.
- **Observed event:** reports something that happened elsewhere.
- **Effect result:** reports the outcome of external work.

A transition produces:

```text
next state
events
commands
effects
reply or rejection
```

Closed alternatives are represented with algebraic data types, enums, sealed variants, tagged constructors, or the closest idiomatic equivalent in the implementation language.

Small pure decisions may use a stateless decision machine. Behavior involving order, waiting, retries, confirmation, recovery, or reconciliation uses meaningful lifecycle states.

### Effects: The Outside World Is Explicit

Pure transitions do not read files, call networks, start processes, inspect clocks, or generate randomness. They emit declared effects instead.

For example:

```text
transition emits WriteFile
        |
        v
effect executor performs the write
        |
        v
FileWritten or FileWriteFailed
        |
        v
result returns to the machine
```

The machine owns sequencing, retries, compensation, and state progression. The executor performs one request and returns one typed result.

This turns hidden failures into diagnosable facts: rejected commands, illegal transitions, failed effects, stale states, or violated laws.

### Runnable Surfaces: Thin Entry Points

A CLI, browser UI, mobile view, HTTP route, batch job, or executable is a runnable surface. It must:

1. parse outside input;
2. construct a declared command or typed rejection;
3. delegate through the module's public boundary;
4. render the resulting reply or rejection.

Runnable surfaces may execute declared boundary effects. They must not duplicate domain decisions or import private machine internals.

## 3. Prove the Result

RMS checks that the declared semantics are represented and reachable in code:

- module dependencies use public contracts;
- declared states and variants exist;
- stateful transitions accept state and classified input;
- pure roles contain no hidden I/O;
- effects have executors and typed results;
- effect results return through transitions;
- runnable surfaces reach the declared machine driver;
- properties execute their declared operation and oracle;
- traces come from real transition records;
- reusable packages match committed source.

During development, the agent runs:

```bash
rms gate --root .
```

For a committed production candidate, it runs:

```bash
rms audit --root . --strict
```

Strict audit does not merely trust evidence files. It regenerates deterministic smoke traces and properties from the committed implementation, compares them with the claimed evidence, rebuilds reusable packages, and fails if proof commands modify production files.

## Agentic and Deterministic Work

RMS combines language-model judgment with deterministic enforcement.

| Agentic work | Deterministic RMS work |
| --- | --- |
| Understand natural-language intent | Validate canonical artifacts |
| Ask necessary product questions | Check module and dependency boundaries |
| Surface edge cases and impossible outcomes | Validate variants and transitions |
| Propose laws, contracts, and machines | Check effects, roles, and public surfaces |
| Write idiomatic business logic | Execute properties and regenerate traces |
| Diagnose nuanced semantic smells | Detect drift, stale evidence, and provenance gaps |

The agent interprets meaning and writes code. RMS constrains the shape and mechanically checks the result. Provider-generated plans remain advisory until represented in canonical RMS artifacts.

## The Main Artifacts

Each artifact has a distinct responsibility:

- `module.yaml`: what a module owns, promises, requires, and must prove;
- `implementation.yaml`: how those semantics map to files, symbols, machines, effects, surfaces, and proof commands;
- source files: implementations of RMS-declared roles;
- `verification/`: executable evidence for laws, contracts, properties, traces, boundaries, and recovery;
- `AGENTS.md`: instructions that teach coding agents to work through the RMS gate.

The manifests are architectural source of truth. Source code is their language-specific realization. Evidence proves that the two agree.

## Seeing The Semantic Layer

Canonical artifacts are designed for deterministic tools, but people should not need to read every YAML file to understand a system. RMS includes an experimental local explorer:

```bash
rms view --root . --watch
```

It presents the same committed semantics through five questions: What exists? How does behavior flow? What should change? Where did a bad state first appear? What proves the claims? The viewer connects modules, contracts, machines, effects, traces, evidence, diagnostics, and source references while keeping unknown links visible as gaps.

The explorer is deliberately read-only. It derives its model from canonical artifacts and cannot edit them, so a convenient visual projection never becomes a competing source of truth.

## The Normal Workflow

```text
1. User describes the product in natural language.
2. Agent asks only questions needed to resolve meaningful ambiguity.
3. Agent uses RMS to design the module composition.
4. Agent applies laws, contracts, machines, effects, and evidence obligations.
5. RMS creates the declared implementation roles.
6. Agent fills those role bodies with idiomatic code.
7. RMS executes native tests, properties, traces, and package checks.
8. Agent runs the development gate.
9. Agent commits the candidate.
10. Strict audit proves the committed implementation.
```

Commands such as `rms spec apply`, `rms machine apply`, and `rms surface apply` are primarily agent-facing. Users should not need to manually author semantic change objects during ordinary product work.

## What RMS Is Not

RMS is not:

- a required application runtime;
- a programming language;
- a replacement for Rust, Swift, JavaScript, Python, or their native tools;
- a rigid universal folder layout;
- a generic boilerplate generator;
- a claim that tests can prove every possible program property.

RMS is a semantic architecture and executable verification layer around ordinary software. Canonical meaning remains language-agnostic; bindings realize and inspect it idiomatically.

## In One Sentence

You describe what you want; an agent uses RMS to encode the meaning, generate the right structure, fill in the implementation, and prove that the resulting software matches its claims.

For operational command and schema details, see [README.md](README.md), [TOOLING.md](TOOLING.md), and [MANIFEST.md](MANIFEST.md).

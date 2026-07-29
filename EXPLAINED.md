# RMS, Explained

Software can now be changed faster than anyone can recover what it means. A coding agent can produce a convincing patch in minutes, yet leave the next person—or the next agent—to rediscover who owns the behavior, which failures matter, and whether the tests prove anything beyond the examples they contain.

The code may work. The system becomes harder to understand.

Reliable Modular Systems (RMS) is designed for that gap. It gives software built by agents a durable account of its meaning, structure, and evidence. The human still decides what the product should do. The agent still writes the code. RMS keeps those decisions from dissolving into implementation details and forgotten conversations.

The simplest way to understand RMS is as a **semantic build system**.

A conventional build system turns source code into a runnable artifact. RMS turns accepted product meaning into owned behavior, declared implementation roles, and executable evidence.

```text
Conventional build: source code → runnable artifact

RMS: product intent → owned meaning → declared structure
                    → implementation → observed proof
```

RMS does not run inside the finished application. It surrounds the work of changing that application, much as a compiler, build system, and test runner surround ordinary source code.

## Follow One Change

Imagine a product owner asks:

> When a payment times out, retry it twice and then require manual review.

The request sounds small. An agent could find a retry loop, change a number, add a test, and finish. But the sentence contains several decisions that matter beyond that loop:

- Does “twice” mean two attempts in total or two retries after the first attempt?
- Is a timeout different from a declined payment?
- What happens if a late success arrives after manual review has begun?
- Who is allowed to place a payment into review?
- Is retry timing a product rule or provider-specific behavior?
- What evidence would demonstrate that no third retry can occur?

These are not coding details. They are part of what the product means.

RMS makes the change pass through five stages before it can be considered complete.

### 1. Turn the Request Into Explicit Facts

The first stage is not architecture. It is understanding.

For a natural-language request, an AI provider extracts a small typed description: this change involves a payment-recovery lifecycle, an external payment effect, a decision about retries, and an unresolved question about attempt counting. Exact statements from the request remain exact statements; inferences carry their rationale; material uncertainty remains a question.

This description is called **typed intent**. Its purpose is to make interpretation inspectable. It cannot name the owning module, decide how modules should be divided, recommend files, or invent an architecture.

If the meaning is unclear, RMS stops here. Asking whether “retry twice” means two or three total attempts is better than silently embedding one interpretation in code.

### 2. Find the Owner

Once the intent is valid, RMS compares the concepts named in it with the ownership already declared by the project.

Perhaps a payment-recovery module owns timeout policy and a payment-provider adapter owns network communication. RMS can then distinguish the decision—whether another attempt is allowed—from the effect—asking the provider to perform that attempt.

If one owner is supported by the project’s authoritative records, RMS selects it. If several modules match equally, or the relevant boundary has never been modeled, RMS selects no owner. A nearby file, a familiar language, or a plausible candidate is not enough.

This is an important form of restraint. Uncertainty does not become architecture merely because an agent can continue coding.

### 3. Declare the Meaning Before the Code

Before implementation, the accepted behavior becomes part of the project’s durable semantic record. For the payment example, that might include:

- the states in which another attempt is legal;
- the exact retry limit;
- timeout, success, decline, and late-result outcomes;
- the transition into manual review;
- the external payment request and its possible results;
- the evidence expected to prove the retry ceiling and recovery path.

These declarations live with the repository rather than in the conversation that produced them. RMS calls them **canonical artifacts**: the project-approved record of its meaning. They give future agents and reviewers a stable answer to “what is this supposed to do?”

Within the part of a project managed by RMS, these accepted records are the architectural source of truth. Source code realizes them; it does not quietly redefine them.

### 4. Fill Declared Roles

RMS gives the implementation a shape without dictating every class, function, or folder.

The retry decision belongs in a pure transition: given the current recovery state and a timeout result, decide whether to request another attempt or enter manual review. The provider call belongs in an effect executor: perform one payment request and report one typed result. A UI or API surface translates outside input into the public command and renders the result; it does not repeat the retry policy.

The agent remains free to write idiomatic Rust, Swift, JavaScript, Python, or another language. Its freedom is bounded by meaning rather than by a universal directory template.

### 5. Prove the Observed Result

Finally, RMS checks the path from promise to implementation to evidence.

For this change, useful proof might execute the real transition through successive timeout results and show:

```text
first timeout  → request retry
second timeout → request retry
third timeout  → require manual review
late success   → follow the declared reconciliation rule
```

The important word is **execute**. A document saying “retry limit is tested” is a declaration. A generic test command is not automatically proof of every property. RMS distinguishes evidence that was declared from validators and commands it actually observed during the check.

While building or diagnosing a machine, an agent can use `rms probe` to send a command, observed event, effect result, or ordered sequence through the implementation's real transition-record path and inspect the resulting states, cases, replies, rejections, and emitted work. A probe is fast diagnostic feedback: it does not execute effects and does not become verification evidence.

## AI Interprets; RMS Determines Authority

RMS combines language-model judgment with deterministic enforcement, but their responsibilities are deliberately different.

```text
AI proposes typed facts.
RMS validates them.
The project determines ownership.
RMS selects the route.
The agent implements it.
Evidence proves the observed result.
```

The AI provider is a constrained interpreter at the entrance. It is good at understanding a human request, noticing ambiguity, and expressing responsibilities in consistent language. It is not allowed to make its proposed architecture authoritative.

RMS then performs the parts that should not depend on a model’s confidence: validating the extracted facts, matching them against canonical ownership, selecting the permitted kind of change, and refusing ambiguous routes.

AI is convenient, not fundamental. A human, CI system, or offline tool can provide the same typed intent directly. What RMS requires is explicit intent, not a particular model.

If provider execution fails, RMS records the failure and authorizes nothing. If the intent is valid but materially uncertain, RMS asks for clarification. If ownership is unresolved, RMS selects no owner. An agent should not manufacture a convenient interpretation to get past any of these outcomes.

This boundary is what makes AI useful without making it sovereign.

## Structure That Survives the Conversation

RMS organizes durable meaning at two scales.

A **module** owns one coherent capability. It states what it owns, what it promises publicly, what it needs from other modules, which external effects it may request, and what evidence its promises require. Modules interact through public contracts rather than each other’s private representations and helpers.

Inside a module, behavior is expressed as a **machine**. The name is less exotic than it sounds: a machine is simply an explicit account of how inputs produce outcomes.

```text
current state + input → next state + outputs
```

An input may be a command, something observed elsewhere, or the result of an external effect. An output may include a reply, rejection, event, command, or request for external work.

This shape is useful even for small decisions. For order-dependent behavior—waiting, retries, confirmation, cancellation, recovery—it prevents the lifecycle from dissolving into loosely related booleans, callbacks, and nullable fields.

External work remains explicit. A pure decision does not secretly call a network, read a clock, write a file, or retry a request. It asks for a declared **effect**. An executor performs that one request and returns a typed result to the machine. The machine, not the executor, owns what happens next.

The edge of the application—a CLI, mobile screen, HTTP route, browser, or batch job—is a **runnable surface**. It parses outside input, delegates through the public boundary, and renders the outcome. It does not become a second home for product policy.

These distinctions make failures legible. A problem can be identified as an invalid command, illegal transition, failed effect, stale result, broken boundary, or violated law instead of appearing as undifferentiated “application behavior.”

## A Checked Change Ticket

When RMS has selected both an owner and the kind of change required—a ready **route**—it issues a route receipt. The receipt is like a checked change ticket attached to the semantic work.

It binds the task to the repository, current revision, selected owner, intended target, and permitted family of RMS changes. An agent cannot route a payment-retry task and use that receipt to redesign an unrelated account module. A commit, repository mismatch, different target, tampering, or unsupported action invalidates it.

The receipt is a procedural integrity control, not a security boundary. It does not grant permission to edit arbitrary source files, make Git commits, deploy software, or bypass the host’s authority. It is not a filesystem sandbox. It proves that an RMS-managed change to meaning or module structure follows the route that was actually issued.

## Proof Without Pretending

Software assurance is often weakened by an imprecise word: “passed.” Passed what, over which code, using which evidence?

RMS makes the scope visible.

During development, a change check runs the proof relevant to the current candidate. After an authorized commit, a stricter audit can use the commit as a stable source identity, regenerate deterministic evidence, rebuild reusable packages, and detect proof commands that unexpectedly modify production files.

RMS also distinguishes repository coverage. A project adopted gradually may contain several RMS modules surrounded by legacy code that RMS does not yet own. In that case, a successful check means the selected modules and their declared dependencies passed. It does not certify the entire repository. Complete coverage is a stronger, explicit claim and is rejected while production paths remain outside RMS ownership.

This is not modest wording around a larger guarantee. It is the guarantee: the selected scope passed the proof that was actually observed.

## Why Agents Benefit

Coding agents are unusually strong at local transformation. Give an agent a file, a failing test, and a clear outcome, and it can often move quickly. The harder problem is preserving coherence across many such transformations.

Without a durable semantic layer, an agent can easily:

- put behavior in the nearest convenient file rather than its true owner;
- copy a decision into a UI or adapter;
- change source before noticing that a public promise changed;
- invent a new abstraction when an existing capability already owns the concern;
- hide sequencing and retry policy inside effectful code;
- treat a declared test or generic verification command as observed proof;
- report repository-wide success after checking only one part;
- leave the next agent to reconstruct the reasoning from implementation accidents.

RMS does not replace the agent’s judgment. It gives that judgment a disciplined path and a durable result. The agent interprets, proposes, implements, and diagnoses. RMS validates ownership, controls changes to declared meaning, checks structural correspondence, executes declared proof, and reports the observed scope.

The result is not less agentic software. It is software in which many agents can contribute without each one becoming a temporary architect with no shared memory.

## Where RMS Is Worth the Weight

RMS is most useful when the cost of architectural drift is higher than the cost of making meaning explicit:

- long-lived products;
- systems maintained by several people or agents;
- multiple modules, applications, or integration boundaries;
- asynchronous workflows and external effects;
- public contracts and compatibility promises;
- reusable capabilities;
- domains where failure and recovery behavior matter.

A disposable script may not need RMS. A small pure library may need only one straightforward module. RMS is not a reason to split software mechanically; it is a way to preserve the boundaries and promises that are genuinely present.

## What RMS Cannot Do

RMS is only as complete as the project modeled in it. If an important production boundary has not been adopted, RMS cannot responsibly assign its owner or certify its behavior.

RMS does not prevent an agent from editing files directly. Its receipts govern RMS-managed changes to meaning and module structure; project guidance, review, native tools, and host policy still govern source edits and Git authority.

RMS cannot prove behavior that its executors did not run. It can report declarations, structural validation, generated properties, traces, native verification, and package checks honestly, but no label turns unobserved behavior into evidence.

Nor does RMS replace product judgment, security analysis, performance engineering, incident readiness, or operational experience. It gives those concerns durable places to be stated and tested. People remain responsible for deciding which promises matter.

## The Mental Model

RMS is the project’s architectural memory and semantic build system.

The human describes the desired outcome and resolves meaningful ambiguity. AI helps translate that intent into constrained facts. Canonical artifacts preserve accepted meaning and ownership. RMS selects and binds the permitted semantic route. The agent fills declared roles with idiomatic code. Executable evidence proves only the scope that was actually observed.

That is the division of labor:

```text
Human: decide what should be true
AI:    help interpret the request
RMS:   preserve meaning, ownership, and proof
Agent: implement the declared behavior
```

For installation and everyday commands, see [README.md](README.md). For precise command behavior and result types, see [REFERENCE.md](REFERENCE.md) and [TOOLING.md](TOOLING.md). For the normative model, see [SPEC.md](SPEC.md) and [MANIFEST.md](MANIFEST.md).

# Designing RMS for Understandability

RMS exists to preserve meaning across software changes. Its own design must
therefore be understandable enough that a maintainer can predict its behavior,
explain its refusals, and extend one concern without reconstructing the whole
tool.

Understandability is an architectural property, not a prose layer added after
the architecture is complete. RMS evaluates a design partly by the amount of
state a person or agent must hold in mind:

- the number of independent concepts and choices;
- the combinations those choices permit;
- the distance between a public promise and its owner, realization, and proof;
- the amount of unrelated machinery required to explain or change one concern;
- the number of histories that lead to observably different results.

Executable properties follow the same principle. RMS exposes one small vocabulary of observations, predicates, temporal expressions, quantities, verdicts, and analysis artifacts. Evaluation, search, relationship analysis, replay, monitoring, and the Explorer all project that vocabulary instead of inventing command-specific meanings.

Nondeterministic schedules are explored through one deterministic breadth-first policy. “Choose any” remains understandable because every choice is treated uniformly, witnesses are shortest, counterexamples are minimized, and incomplete exploration is never called proof.

Completeness does not excuse unnecessary state. Explicitly modeling five
independent booleans still creates thirty-two combinations.

## Understandability Laws

These laws guide the RMS implementation and its public projections. They do not
add new canonical vocabulary to downstream RMS projects.

### Locality

One concern should be explainable from its owner, public contract, machine or
semantic operation, declared effects and dependencies, implementation binding,
and evidence. Unrelated command families and module internals should not be
prerequisites.

### Uniqueness

Every meaning-affecting decision has one canonical owner and one authoritative
source. Reports, viewers, prompts, caches, traces, and packages are projections
or evidence; they do not become alternate owners.

### Contiguity

A claimed public behavior has an unbroken, inspectable path:

```text
promise
→ contract
→ semantic owner
→ operation or machine case
→ implementation binding
→ executable evidence
```

A missing edge remains visible as a gap. RMS must not bridge it with naming
similarity, conversational confidence, or an implementation accident.

### Representability

Invalid combinations should be unrepresentable where the implementation
language permits it. Prefer tagged variants and validated constructors over a
status field accompanied by conditionally meaningful optional fields.

Validation remains necessary at external boundaries, but internal code should
not repeatedly re-prove facts its types can preserve.

### Progress

Every non-ready workflow state identifies one principal unmet condition and one
next action. Candidates and secondary diagnostics may explain the result, but
must not create competing implied routes.

### Explicit Choice

A choice that changes ownership, meaning, compatibility, authority, or proof
scope is deterministic or remains explicitly unresolved. RMS never breaks such
a tie arbitrarily.

Choices may branch when every branch is intentionally part of proof
exploration. Exploration uses stable ordering, records every material decision,
and produces deterministic replay. Random or generated exploration records its
seed. Randomness is proof input, never semantic authority.

## State-Space Review

A material RMS design change should include a short state-space delta:

| Question | Expected answer |
| --- | --- |
| What new state or choice is introduced? | Name the semantic distinction, not its storage field. |
| Which existing states can it combine with? | List meaningful combinations and exclude impossible ones structurally. |
| What old distinctions does it remove or derive? | Prefer a smaller authoritative model with projections. |
| What is deterministic? | Name the ordering, selection rule, or canonical representation. |
| What may branch? | State why the choices are equivalent or intentionally explored. |
| How is failure replayed? | Name the stable input, seed, schedule, or counterexample. |
| Can the concern be understood locally? | Name the bounded source and evidence paths. |

Do not add a concept merely because it can be represented. Add it when it
changes observable meaning and cannot be derived from existing state.

## Comprehension Evidence

Mechanical conformance is necessary but does not establish that RMS is
understandable. Maintainer evaluation should also test whether a reader who did
not implement a feature can answer:

1. What owns this behavior?
2. Why did RMS select, refuse, or defer this route?
3. Which outside effects can occur?
4. What exactly does the reported proof cover?
5. What must be understood to extend this behavior safely?

The semantic explorer should answer the corresponding system questions from one
derived graph. Blind walkthroughs and clean-room changes are stronger evidence
than explanations written by the feature's author.

## Foundation for a Future RMS-in-RMS Rewrite

The current Rust CLI remains the independent reference implementation. RMS
artifacts describing RMS are test subjects and architectural records; they do
not govern ordinary RMS maintenance unless the user explicitly requests
self-hosting.

Semantic revisions made during ordinary RMS maintenance are sealed by the
repository maintainer workflow. This authority is a closed, explicitly
declared self-application variant: it is valid only for the RMS
self-development module, retains immutable change-record and canonical
projection digests, and is rejected for ordinary downstream modules. The seal
records maintainer authority; it does not claim that the candidate RMS binary
authorized or certified its own change.

A later rewrite should be evolutionary rather than a flag-day replacement. The
target seams are:

```text
typed intent
→ ownership resolution
→ workflow state
→ semantic mutation plan
→ proof selection and aggregation
→ public projection
```

Each seam should become a pure, closed transition with boundary effects outside
it. Repository discovery, provider calls, filesystem writes, process execution,
Git inspection, and publication remain explicit adapters.

The first extracted seam is `tooling/rust/rms/src/workflow.rs`. It owns the
closed public action phases, authorization states, and the command/manual action
sum type while preserving the existing `rms.surface/v2` representation.

Before RMS can safely replace substantial parts of its implementation through
RMS, the repository should have:

- a construction-safe workflow and ownership state model;
- one shared outcome/status algebra with explicit projections for specialized
  domains;
- contiguous public behavior paths with no implicit edges;
- deterministic golden traces for routing and proof selection;
- replayable exploration for schedules, generated inputs, and faults;
- wire-compatibility fixtures for every supported public schema;
- an independent native oracle capable of comparing old and replacement
  behavior;
- a bootstrap and recovery path that does not require the candidate
  implementation to certify itself.

Self-hosting is useful evidence only while an independent oracle remains
available. A system declaring itself correct is not additional proof.

## Reduction Sequence

The first two reductions are implemented without changing the public
`rms.surface/v2` shape:

1. Public follow-up actions use distinct opaque command and manual variants.
2. Ownership resolution is `Selected(module) | Ambiguous | None | Invalid`;
   only the selected variant can carry a module.

The next promising reductions are:

1. Intent facts: known explicit facts, known inferred facts, and material
   unknowns should carry only the evidence valid for that variant.
2. Status projection: specialized status vocabularies should derive from a
   small shared algebra without erasing domain-specific meaning.
3. Semantic paths: complete and gapped paths should be distinct variants, with
   the first missing edge carried by the gapped form.

Each reduction should preserve public compatibility unless a separately
versioned exchange-format change is justified.

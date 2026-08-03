# Property: Probe Product-State Expressions

Property: every accepted v0.3 state expression is closed, typed, evaluated over
the declared current instance-state projections, and replayable when false.

Input space: scalar and closed-variant observations; arithmetic and Boolean
expressions across several instances; all check timings; invalid identifiers,
instances, pointers, references, and types; bounded optimistic-concurrency,
duplicate-delivery, ordering, deadlock, and frame fixtures.

Operation: validate and compile the assembly, explore the bounded product state,
minimize any failing schedule, and replay the counterexample through the same
probe engine. The frame fixture also evaluates the bad transition record through
the existing behavioral-contract frame checker.

Oracle:

- v0.1 and v0.2 behavior is unchanged and the new assertion is rejected;
- invalid v0.3 expressions fail before exploration;
- missing or ill-typed runtime projections invalidate the run;
- false expressions retain structured observed facts and exact replay;
- safe optimistic commits and idempotent duplicate delivery exhaust their bounds;
- lost update, invalid ordering, productive deadlock, and frame mutation yield
  their intended shortest replayable counterexamples;
- the frame checker assigns provider blame at `/data/owner`.

Runner: `src/probe/mod.rs#state_expression_concurrency_acceptance_corpus_is_replayable`.

Counterexamples: `verification/fuzz/counterexamples`.

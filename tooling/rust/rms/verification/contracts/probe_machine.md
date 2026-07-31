# Contract Evidence: probe-machine

Covered by native Rust tests, CLI smoke tests, schema fixtures, topology fixtures, and maintained Rust, Swift, JavaScript, and Python adapter roundtrips.

- Existing describe, inline, and file probes call the exact transition-record path, chain `state_after`, validate canonical cases, and report the first expectation failure.
- v0.2 adapters batch independent `{state,input}` evaluations; v0.1 adapters retain the one-transition fallback.
- Assembly fixtures exercise series, fan-in, fan-out, cycles, simultaneous commands, repeated modules, and five-instance slices.
- v0.2 workload fixtures derive only public command examples, enforce per-action budgets, and record the exact normalized injection in replay decisions; v0.1 assemblies remain unchanged.
- Route resolution accepts canonical dependency bridges and protocols while rejecting missing, invented, incompatible, or ambiguous wiring before machine execution.
- Breadth-first exploration uses stable ordering, virtual time, explicit substitutes and faults, global-state deduplication, and returns `inconclusive` rather than `pass` at a bound.
- Every failure preserves the exact failing check while reducing stimuli, fault decisions, delays, and payloads; replay distinguishes resolved (`0`), reproduced (`1`), and invalid (`2`).
- Guided hunt exploration uses seeded semantic novelty, continues after a failure, preserves distinct checks, and targeted replay reaches each retained failure even when another check fails earlier on the same path.
- System traces retain source provenance, local transitions, envelopes, routes, correlation, causation, idempotency, protocol state, faults, checks, coverage, bounds, mode, and exhaustion.
- The Lawbook renders canonically referenced probe evidence as a plain causal timeline.
- Structural tests reject adapters that call a driver or effect executor, bypass the transition-record function, or omit the temporary-file protocol.

Probe output is an ephemeral diagnostic unless canonical verification explicitly references it. Referenced assemblies are rerun and must exhaust successfully; referenced counterexamples must replay as resolved.

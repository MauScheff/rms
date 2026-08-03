# Probe Product-State Expressions Are Closed and Replayable

Promise: `rms/probe-assembly/v0.3` may evaluate one closed expression over typed
RFC 6901 projections from the current states of named assembly instances.

The assembly validator rejects duplicate observation IDs, unknown instances,
invalid pointers, unknown expression references, and incompatible term types
before exploration. The probe engine compiles each expression once. At check
time it projects the declared values from `World.states`. A missing path or
runtime type mismatch makes the run invalid. A false expression is a normal
check failure with a minimized replayable counterexample and a structured
observed name/value map. Complete instance states remain in the system trace.

The scheduler, transition atomicity, machine model, and solver boundary are
unchanged. Assembly v0.1 and v0.2 remain valid and reject the new assertion.

Command/tool: `cargo test --workspace --locked state_expression_`.

Expected result: valid v0.3 expressions evaluate at `always`, `quiescent`, and
`within`; invalid definitions fail before exploration; unsafe product-state
fixtures minimize and replay; safe bounded fixtures exhaust.

Source provenance: the clean committed candidate revision resolved by
`rms audit --root . --strict`.

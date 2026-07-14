# Property Evidence: cross-module trace causation

Promise: `cross-module-trace-causation` proves `system-traces-preserve-message-causation`.

Input space: recorded protocol envelopes with matching and missing peers, external root causes, internal causes, correlations, endpoints, and sequences.

Oracle:

- each logical envelope has exactly one matching send and receive observation;
- every non-root cause resolves inside the same correlation;
- the first invalid handoff is deterministic.

Realization: `src/main.rs#build_system_trace_report` checks every extracted message observation and causal edge in the finite stitched bundle.

Command/tool: `cargo test -p rms trace_stitch -- --nocapture` and `cargo test -p rms`.

Observed result: the valid handoff passed; the missing receiver identified the first bad handoff; the complete 262-test RMS suite passed.

Source provenance: the clean committed candidate revision resolved by strict audit.

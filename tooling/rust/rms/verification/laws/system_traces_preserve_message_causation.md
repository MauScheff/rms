# Law Evidence: cross-module trace causation

Promise: `system-traces-preserve-message-causation`.

Scenarios: matching sender/receiver observations share one envelope and root cause; a sender with no receiver identifies the first broken handoff.

Command/tool: `cargo test -p rms trace_stitch -- --nocapture` and the full `cargo test -p rms` suite.

Observed result: the valid handoff produced `rms/system-trace/v0.1` with pass; the missing receiver produced `trace.system-handoff-broken`; the 262-test RMS suite passed.

Source provenance: the clean committed candidate revision resolved by strict audit.

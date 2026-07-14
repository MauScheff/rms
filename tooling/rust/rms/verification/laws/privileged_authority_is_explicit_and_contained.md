# Law Evidence: authority containment

Promise: `privileged-authority-is-explicit-and-contained`.

Scenarios: unsafe source is accepted only in a role bound to a declared authority and exact safe facade; the same operation in a pure transition role is rejected.

Command/tool: `cargo test -p rms` (including `authority_check_rejects_unsafe_source_outside_bound_role`).

Observed result: misplaced unsafe source produced `structure.privileged-operation-outside-authority-role`; the 262-test RMS suite passed.

Source provenance: the clean committed candidate revision resolved by strict audit.

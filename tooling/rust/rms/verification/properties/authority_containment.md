# Property Evidence: authority containment

Promise: `authority-containment` proves `privileged-authority-is-explicit-and-contained`.

Input space: inspectable source paths inside and outside declared authority roles, with declared and missing authority ids, safe facades, and evidence.

Oracle:

- an elevated operation inside its bound role and facade is contained;
- undeclared authority or an elevated operation outside the bound role is rejected.

Realization: `src/main.rs#inspect_privileged_source_containment` statically scans each inspectable role path and checks it against canonical authority ownership.

Command/tool: `cargo test -p rms`.

Observed result: misplaced unsafe source produced the expected containment diagnostic and the complete 262-test RMS suite passed.

Source provenance: the clean committed candidate revision resolved by strict audit.

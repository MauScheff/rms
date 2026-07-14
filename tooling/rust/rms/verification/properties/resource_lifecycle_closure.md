# Property Evidence: resource lifecycle closure

Promise: `resource-lifecycle-closure` proves `resource-lifecycles-close-on-terminal-paths`.

Input space: finite product graphs covering acquire, use, release, transfer, illegal operation order, and terminal product paths.

Oracle:

- every reachable resource operation is legal in the current resource state;
- every reachable terminal product state leaves the resource in a declared terminal state.

Realization: `src/main.rs#validate_machine_resource_protocols` explores the reachable product of machine state and each declared resource state.

Command/tool: `cargo test -p rms resource_protocol_ -- --nocapture` and `cargo test -p rms`.

Observed result: acquire/release passed, terminal leakage failed with the expected diagnostic, and the complete 262-test RMS suite passed.

Source provenance: the clean committed candidate revision resolved by strict audit.

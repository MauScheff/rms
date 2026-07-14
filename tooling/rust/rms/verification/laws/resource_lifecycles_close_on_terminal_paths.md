# Law Evidence: resource lifecycle closure

Promise: `resource-lifecycles-close-on-terminal-paths`.

Scenarios: finite product exploration covers acquisition followed by release and acquisition followed by terminal product state without release.

Command/tool: `cargo test -p rms resource_protocol_ -- --nocapture` and the full `cargo test -p rms` suite.

Observed result: acquire/release passed; the leaking path produced `structure.resource-not-closed-on-terminal-path`; the 262-test RMS suite passed.

Source provenance: the clean committed candidate revision resolved by strict audit.

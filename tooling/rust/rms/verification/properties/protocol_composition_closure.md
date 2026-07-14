# Property Evidence: protocol composition closure

Promise: `protocol-composition-closure` proves `module-protocols-compose-by-contract`.

Input space: canonical protocol contracts and implementation bindings with complete, missing, duplicate, direction-invalid, and automaton-mismatched participant/message routes.

Oracle:

- all copies of a protocol contract define the same automaton;
- each participant has one implementation owner;
- each message has one sender and one receiver mapping with compatible endpoints.

Realization: `src/main.rs#compose_protocol_contracts` evaluates every participant and message of every discovered protocol identity.

Command/tool: `cargo test -p rms compose_requires_one_protocol_owner_and_route_per_message -- --nocapture` and `cargo test -p rms`.

Observed result: the complete route passed, the missing receiver failed, and the complete 262-test RMS suite passed.

Source provenance: the clean committed candidate revision resolved by strict audit.

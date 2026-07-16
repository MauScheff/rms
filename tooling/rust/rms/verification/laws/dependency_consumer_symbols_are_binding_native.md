# Law Evidence: dependency consumers use binding-native symbols

Promise: an exact dependency consumer reference resolves to either a function or a binding-native type that owns the port.

Scenario: Swift dependency ports owned by a `struct`, `actor`, or `protocol` resolve without weakening exact `path#symbol` checking; an absent symbol remains unresolved.

Command/tool: `cargo test -p rms swift_dependency_consumers_may_be_type_owned_ports`

Expected result: all declared Swift type owners resolve and `MissingPort` does not.

Source revision: strict audit binds this evidence to the committed candidate.

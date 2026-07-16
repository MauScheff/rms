# Law Evidence: external dependency bindings compose directly

Law: `external-dependency-bindings-compose-without-system-duplication`

Promise: an exact `resolution: external` dependency behavior binding satisfies its required capability without duplicating that declaration in system-wide external dependencies.

Scenario: a progressively adopted module binds `account-auth-service` to a local effect port with the same required contract; another undeclared capability remains unresolved.

Command/tool: `cargo test -p rms declared_external_dependency_binding_satisfies_composition`

Expected result: the matching capability and contract are accepted, while a missing binding is rejected.

Source revision: strict audit binds this evidence to the committed candidate.

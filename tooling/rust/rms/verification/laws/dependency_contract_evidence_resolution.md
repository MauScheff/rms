# Law Evidence: dependency contract evidence follows declared paths

Promise: a concrete verification reference whose path names a required capability counts as that dependency contract's evidence, even when its prose uses a human-readable title.

Scenario: `verification/contracts/account_auth_service_v1.md` proves `account-auth-service` while its heading says “Account authentication service.”

Command/tool: `cargo test -p rms dependency_contract_evidence_may_be_linked_by_declared_path`

Expected result: the declared path resolves the semantic edge and no unrelated path can satisfy it.

Source revision: strict audit binds this evidence to the committed candidate.

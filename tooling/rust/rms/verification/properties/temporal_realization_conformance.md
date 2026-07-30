# Property Evidence: temporal realization conformance

Promise: `temporal-realization-conformance` proves `temporal-properties-have-matching-realizations`.

Input space: typed observation sources and values; environment and search-preference assumptions; always, eventually, precedence, exclusion, at-most-once, and bounded-response expressions; valid and invalid unit combinations; removed descriptive fields; and finite semantic, runtime, and platform scopes.

Oracle:

- finite machine, protocol, resource, artifact, and composition scopes require exhaustive or model-checking realization;
- runtime and platform scopes require a declared benchmark, static analyzer, sanitizer, or model checker;
- every predicate reference resolves to a typed observation;
- bounded responses use a quantity dimension compatible with their metric observation;
- descriptive pattern, trigger, condition, and bound fields are rejected.

Realization: `src/property.rs#compile_property` type-checks executable property meaning, and `src/main.rs#validate_temporal_target_report` checks every declared scope and realization pair.

Command/tool: `cargo test -p rms`.

Observed result: focused property-core and conformance tests cover closed expressions, exact unit conversion, dimensional rejection, real-trace evaluation, assumptions, and corpus-only finite-proof rejection. The current complete-suite result is recorded by the candidate audit.

Source provenance: the clean committed candidate revision resolved by strict audit.

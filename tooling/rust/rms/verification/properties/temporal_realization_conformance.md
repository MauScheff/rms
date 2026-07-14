# Property Evidence: temporal realization conformance

Promise: `temporal-realization-conformance` proves `temporal-properties-have-matching-realizations`.

Input space: always, eventually, precedence, exclusion, at-most-once, bounded-response, resource-closure, bounded-resource, finite semantic, runtime, and platform property declarations.

Oracle:

- finite machine, protocol, resource, artifact, and composition scopes require exhaustive or model-checking realization;
- runtime and platform scopes require a declared benchmark, static analyzer, sanitizer, or model checker;
- bounded response declares exactly one complete transition or metric bound.

Realization: `src/main.rs#validate_temporal_target_report` checks every declared temporal target and realization pair.

Command/tool: `cargo test -p rms`.

Observed result: corpus-only finite temporal proof was rejected with `evidence.temporal-realization-mismatch`, and the complete 262-test RMS suite passed.

Source provenance: the clean committed candidate revision resolved by strict audit.

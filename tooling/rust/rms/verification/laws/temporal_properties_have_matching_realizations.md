# Law Evidence: temporal realization conformance

Promise: `temporal-properties-have-matching-realizations`.

Scenarios: a finite `eventually` claim backed only by deterministic examples is rejected; model-checking or exhaustive finite realization is accepted by the semantic validator.

Command/tool: `cargo test -p rms` (including `temporal_property_rejects_corpus_only_finite_proof`).

Observed result: the weak realization produced `evidence.temporal-realization-mismatch`; the 262-test RMS suite passed.

Source provenance: the clean committed candidate revision resolved by strict audit.

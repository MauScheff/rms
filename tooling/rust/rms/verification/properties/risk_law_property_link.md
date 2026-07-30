# Property Evidence: risk law linkage

Promise: `risk-bearing-laws-have-semantic-properties`.

Input space: invariants classified as ordering, safety, bounded, normalization, parser, numeric, or arithmetic, with matching, missing, and unrelated property targets.

Lexical classification uses complete semantic tokens: composition language such
as `boundary` must not be treated as numeric merely because it contains
`bound`.

Oracle:

- the property or fuzz target `proves` the exact invariant id;
- input space, oracle, evidence, realization, and counterexample policy remain explicit;
- evidence prose alone cannot satisfy the property obligation.

Command/tool: `cargo test --workspace --locked risk_bearing_law_requires_matching_semantic_property` and `cargo test --workspace --locked numeric_semantics_detection_does_not_split_boundary_words`.

Expected result: a risk-bearing law without a matching property emits `semantic.law-without-property` and blocks strict audit.

Source provenance: the clean committed candidate revision resolved by strict audit.

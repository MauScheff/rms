# Contract Evidence: analyze-git-impact

Covered by `cargo test --manifest-path Cargo.toml`.

Executable coverage:

- `impact_classifies_contract_and_source_paths` verifies changed paths under module contracts, source roots, and verification contract evidence are mapped to the owning module and classified with conservative review or verification impact.
- `impact_reports_unowned_paths_without_semantic_authority` verifies paths outside discovered RMS module roots remain unowned evidence rather than invented module semantics.

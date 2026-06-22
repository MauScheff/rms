# Contract Evidence: build-module-atlas

Covered by `cargo test --manifest-path tooling/rust/rms/Cargo.toml`, including atlas graph construction and artifact writing tests. The tests verify that atlas nodes are derived from module manifest content, retain stable semantic IDs, include source references and semantic contract clauses, include deterministic trace projections with confidence labels and gaps, and write both `atlas.json` and `index.html`.

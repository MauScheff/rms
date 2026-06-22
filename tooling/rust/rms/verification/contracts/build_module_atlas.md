# Contract Evidence: build-module-atlas

Covered by `cargo test --manifest-path tooling/rust/rms/Cargo.toml`, including atlas graph construction and artifact writing tests. The tests verify that atlas nodes are derived from module manifest content, retain stable semantic IDs, include source references, include deterministic question answers, and write both `atlas.json` and `index.html`.

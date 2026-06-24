# Law Evidence: transition trace

Command:

- `cargo test --manifest-path Cargo.toml`

Covered cases:

- accepted moves alternate X then O;
- accepted moves never overwrite occupied cells;
- a top-row X trace reaches a won status;
- drawn games reject further moves;
- replayed traces expose accepted and rejected outcomes.

Source revision: not recorded for the example scaffold.

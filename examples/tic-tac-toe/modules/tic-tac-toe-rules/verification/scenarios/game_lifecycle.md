# Scenario Evidence: game lifecycle

The game lifecycle is:

```text
InProgress(next: X)
  -> InProgress(next: O)
  -> InProgress(next: X)
  -> Won(winner)
  -> reject all further moves
```

A full board without a winning line reaches `Draw` and rejects all further moves.

Evidence command:

- `cargo test --manifest-path Cargo.toml`

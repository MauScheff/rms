# Law Evidence: semantic shape

Shape: `domain-engine` (pure decisions, closed variants, validated values, transitions, laws, and trace replay)

Evidence:

- `src/representation.rs` defines closed Rust enums for `Command`, `GameStatus`, `Mark`, `MoveRejection`, and `TransitionOutcome`.
- `Cell::new` and `Cell::from_index` make out-of-board cells unrepresentable.
- `src/transition.rs` keeps move application pure: `transition(Game, Command) -> TransitionOutcome`.
- Accepted and rejected lifecycle traces are covered by `verification/laws/transition_trace.md` and `verification/scenarios/game_lifecycle.md`.

Command:

- `cargo test --manifest-path examples/tic-tac-toe/modules/tic-tac-toe-rules/Cargo.toml`
- `rms structure examples/tic-tac-toe/modules/tic-tac-toe-rules/implementation.yaml`

Source revision: local working tree.

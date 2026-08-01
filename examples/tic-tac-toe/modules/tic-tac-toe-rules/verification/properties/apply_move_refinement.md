# Apply-move capability refinement

`apply_move_refines_cli_requirement` exhaustively explores the finite closure of game states reachable through the rules module's public transition. For every reachable state it evaluates all nine validated cells and checks that:

- the result is exactly one accepted reply or declared rejection;
- accepted transitions emit only `MarkPlaced` and rejected transitions emit only `MoveRejected`;
- commands and effects remain empty;
- rejection preserves state; and
- every accepted successor is included in the explored closure.

This discharges the exact external refinement from the CLI module's required `apply-move` capability contract to the rules module's provided contract. The digest-bound `rms/property-analysis/v0.2` record beside this document prevents the evidence from being reused after either contract changes.

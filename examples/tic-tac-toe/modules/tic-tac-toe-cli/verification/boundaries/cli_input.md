# Boundary Evidence: CLI input

Boundary: local command-line input.

Command:

- `sh scripts/smoke.sh`

The parser treats text input as untrusted and only emits a rules command when it can construct a board cell in the finite 3 by 3 board.

Covered cases:

- empty text is rejected as malformed input;
- out-of-board coordinates are rejected before rules delegation;
- `A1` and `b2` parse to board indexes `0` and `4`;
- valid parsed moves are delegated through the rules port;
- the default rules port invokes the local Rust rules bridge, which depends on `tic-tac-toe-rules`;
- domain rejections from the rules engine, such as an occupied cell, are returned explicitly.

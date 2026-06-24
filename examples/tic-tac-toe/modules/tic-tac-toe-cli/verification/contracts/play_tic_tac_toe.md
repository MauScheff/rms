# Contract Evidence: play-tic-tac-toe

Contract: `contracts/play-tic-tac-toe.v1.yaml`

Command:

- `sh scripts/smoke.sh`

Covered cases:

- valid text such as `A1` is delegated to the rules port;
- lowercase coordinates such as `b2` are accepted;
- out-of-board text such as `D4` is rejected before delegation;
- malformed empty input is rejected before delegation;
- the default rules port invokes the Rust rules engine through `rules-bridge/Cargo.toml`;
- occupied-cell rejection is returned from the real rules engine without adding the rejected move to accepted history.

# Law Evidence: semantic shape

Shape: `boundary-adapter` (parsers, boundary validation, ports, effect adapters, and contract or boundary tests)

Evidence:

- `src/representation.mjs` defines tagged constructors for accepted/rejected boundary outcomes.
- `src/parser.mjs` parses untrusted coordinates into finite board-cell commands before delegation.
- `src/adapter.mjs` rejects malformed input before invoking the rules port.
- `src/ports.mjs` keeps the default Rust rules bridge behind an adapter port.
- Boundary and contract behavior are covered by `verification/boundaries/cli_input.md` and `verification/contracts/play_tic_tac_toe.md`.

Command:

- `sh examples/tic-tac-toe/modules/tic-tac-toe-cli/scripts/smoke.sh`
- `rms structure examples/tic-tac-toe/modules/tic-tac-toe-cli/implementation.yaml`

Source revision: local working tree.

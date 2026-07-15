# Evidence: Tic-Tac-Toe CLI delegates apply-move through its declared dependency

Promise:

- apply-move

Scenario:

- The CLI parses a move into its boundary command and delegates the rules decision through `src/ports.mjs#createRulesEffectExecutor`.
- The required `apply-move` contract is resolved to the `tic-tac-toe-rules` provider without importing its private representation or transition roles.

Command/tool:

- `rms compose --root examples/tic-tac-toe`
- `rms spec check examples/tic-tac-toe/modules/tic-tac-toe-cli/module.yaml`
- `rms verify examples/tic-tac-toe/modules/tic-tac-toe-cli/implementation.yaml`

Expected result:

- Composition resolves required capability `apply-move` to `tic-tac-toe-rules` with a matching contract.
- The dependency behavior binding resolves its exact consumer symbol and provider module.
- Accepted and rejected move outcomes remain explicit boundary effect results and replay records.

Source revision: resolved from the candidate Git commit by `rms audit --root . --strict`.

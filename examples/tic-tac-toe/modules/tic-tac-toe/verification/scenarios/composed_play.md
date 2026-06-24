# Scenario Evidence: composed play command

Promise:

- The `tic-tac-toe` composite module exposes `play-tic-tac-toe`.
- The public command is exported from the internal `tic-tac-toe-cli` child.
- The CLI adapter depends on the internal `tic-tac-toe-rules` child for rule decisions.

Evidence:

- `rms compose --root examples/tic-tac-toe` verifies containment, internal visibility, and parent export backing.
- `rms verify examples/tic-tac-toe/modules/tic-tac-toe/module.yaml` rolls up composition and child implementation verification.

Source revision: recorded by the verifier or conformance report at runtime.

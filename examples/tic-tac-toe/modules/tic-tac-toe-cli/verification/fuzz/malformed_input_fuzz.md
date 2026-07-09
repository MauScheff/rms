# Fuzz Evidence: malformed CLI input

Promise:

- Fuzz target `tic-tac-toe-cli-malformed-input-stops-before-domain` proves the boundary parser rejects malformed local input before delegating to the rules module.

Input space:

```yaml
raw_cli_input:
  - empty string
  - out-of-board coordinates such as D4
  - valid mixed-case board coordinates such as A1 and b2
```

Oracle:

- malformed input returns a typed boundary rejection
- malformed input does not delegate to the rules module
- accepted input delegates only parsed board cells

Command/tool:

- `rms property run implementation.yaml --profile smoke`
- Binding command: `node tests/boundary-smoke.mjs` from this module.

Expected result:

- Empty input and out-of-board coordinates are rejected.
- Valid coordinates delegate board indexes 0 and 4 to the rules port.
- Future generated failures should be recorded under `verification/fuzz/counterexamples` with `spec: rms/property-counterexample/v0.1`.

Source revision: recorded by git commit or strict audit provenance before production use.

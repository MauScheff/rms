# Contract Evidence: probe-machine

Covered by the RMS CLI test suite and maintained Rust, Swift, and JavaScript binding roundtrips.

- `rms probe implementation.yaml --describe` validates the handshake and payload schemas.
- Inline and file probes call the exact transition-record path, chain `state_after`, validate canonical cases, and report the first expectation failure.
- The Tic-Tac-Toe dogfood probe reaches `Won` through five `PlaceMark` commands.
- The RMS workbench dogfood probe observes `ApplySemanticChange` followed by `SemanticChangeRecordWritten` without running the write executor.
- Structural tests reject probe adapters that call a driver or effect executor, bypass the transition-record function, or omit the temporary-file protocol.

Probe output is ephemeral diagnostic evidence unless `--out` is supplied. It never satisfies declared trace, scenario, property, or release evidence.

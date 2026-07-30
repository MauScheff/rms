# Contract Evidence: `hunt-rms-system`

Promise:

- `rms hunt` discovers risk-derived nightly realizations and canonical probe explorations.
- It runs from an exact clean revision in an isolated checkout, checkpoints resumably, and records tools, seed, budget, commands, coverage, findings, and proof scope.
- Behavioral findings are retained only after replay.
- Finite exhaustion and bounded empirical evidence remain visibly distinct.

Evidence:

```bash
cargo test -p rms hunt --no-fail-fast
cargo test -p rms --test cli_smoke hunt_runs_nightly_lane_in_an_isolated_checkout_and_resumes
```

The CLI smoke fixture commits a tiny project, executes a nightly lane through the four hunt environment variables, validates its lane result, resumes the completed run, and confirms that the original tracked checkout remains clean.

Source revision: resolved from the committed candidate by strict audit.

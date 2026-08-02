# Contract Evidence: `hunt-rms-system`

Promise:

- `rms hunt` discovers risk-derived nightly realizations and canonical probe explorations.
- It runs from an exact clean revision in an isolated checkout, checkpoints resumably, and records tools, seed, budget, commands, coverage, findings, and proof scope.
- One absolute execution deadline bounds every phase and worker. Each lane, guided-search loop, subprocess, minimization attempt, and replay receives only its remaining share; timed-out subprocess groups are terminated.
- Completed lanes checkpoint incrementally. A resumed run counts prior elapsed execution time, preserves partial artifacts from interrupted `running` lanes, and explicitly reruns those lanes instead of silently accepting or discarding partial output.
- Behavioral findings are stable, deduplicated, and retained only after targeted replay; guided lanes continue to a fixed distinct-finding cap.
- Explicit finite exhaustion and bounded empirical or guided evidence remain visibly distinct.
- The finding-first v0.2 report retains the shortest replay and remains readable beside historical v0.1 reports.

Evidence:

```bash
cargo test -p rms hunt --no-fail-fast
cargo test -p rms --test cli_smoke hunt_runs_nightly_lane_in_an_isolated_checkout_and_resumes
cargo test -p rms hunt::tests::guided_hunt_deadline_bounds_slow_probe_and_kills_its_process_group
cargo test -p rms hunt::tests::interrupted_resume_preserves_partial_artifacts_and_explicitly_reruns_lane
```

The CLI smoke fixture commits a tiny project, executes a nightly lane through the four hunt environment variables, validates its lane result, resumes the completed run, and confirms that the original tracked checkout remains clean.

Source revision: resolved from the committed candidate by strict audit.

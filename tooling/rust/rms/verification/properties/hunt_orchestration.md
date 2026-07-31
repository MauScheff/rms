# Property Evidence: Hunt Orchestration

Property: `risk-derived-hunt-lanes-are-required`

The generated fixture crosses boundary and reusable semantics with generated, coverage-fuzz, mutation, and focused-exception declarations. The oracle requires the exact missing obligation diagnostics and accepts only the closed, focused exceptions that discharge them.

Property: `hunt-outcomes-preserve-proof-scope`

The hunt core classifies replayable behavioral findings as bugs, surviving mutants as proof gaps, passing open-ended lanes as bounded clean evidence, and only explicitly exhaustive strategies with recorded exhaustion as finite proof scope. Guided lanes never become proof. Stable semantic IDs deduplicate recurring failures, sum occurrences, and retain the shortest replay. Seeded guided exploration continues after a check failure and targeted replay reaches each distinct retained failure. Lanes that never start contribute no bounded evidence. Independent smoke baselines use configured parallelism, while trace regeneration and mutation remain serialized.

Commands:

```bash
cargo test -p rms hunt_posture_derives_boundary_and_reusable_obligations_with_closed_exceptions
cargo test -p rms hunt_outcomes_distinguish_bugs_gaps_and_bounds
cargo test -p rms hunt_scope_excludes_unstarted_lanes_and_baselines_use_parallelism
cargo test -p rms v2_findings_have_stable_ids_and_deduplicate_to_the_shortest_replay
cargo test -p rms guided_exploration_is_seeded_and_keeps_distinct_replayable_findings
```

Source revision: resolved from the committed candidate by strict audit.

---
name: hunt-bugs
description: Find, minimize, and replay software bugs with RMS proof exploration, generated properties, fuzzers, sanitizers, mutation testing, and unattended hunts. Use for requests to find bugs, fuzz, harden, soak, run overnight checks, test reliability, or assess proof strength.
---

# Hunt Bugs

When the repository provides `FIRST_BUG_HUNT.md` and the user is learning or dogfooding the workflow, use its runnable teaching hunt before beginning an open-ended campaign.

1. Require a clean committed source revision, then inspect the selected module closure with `rms hunt --root <root> --dry-run` to see exact lanes, inherited composite-child ownership, tools, budgets, exclusions, and unsupported obligations. Use repeatable `--profile <profile>` and `--lane <id|kind|strategy>` selectors for a partial campaign. A selector excludes unrelated lanes; it does not make them invalid. Dry runs use the same committed revision and reproducibility checks as execution; if refused, inspect the changed paths named by RMS.
2. Treat proof methods honestly:
   - exhausted finite exploration proves only the declared finite model;
   - generated cases, fuzzing, sanitizers, mutation testing, benchmarks, and bounded searches are evidence, never a global bug-free claim;
   - a reached search bound is inconclusive.
3. Use `rms hunt --root <root> --budget 8h` for an unattended run; add `--module <module.yaml>` only for an intentionally bounded closure and `--seed <n>` for a repeatable campaign.
4. Declare nightly realizations with exact commands and runners. Use generated properties for pure decisions and numeric extremes, coverage fuzzers for untrusted boundaries, exhaustive/model-checker or probe exploration for stateful behavior, schedule/fault exploration for distributed behavior, sanitizers/static analyzers for unsafe boundaries, and mutation testing for important oracles. Declared probe assemblies automatically gain one seeded semantic-novelty lane; optional v0.2 public-example workloads expand inputs without adding CLI flags.
5. Make each nightly runner honor `RMS_HUNT_SEED` and `RMS_HUNT_BUDGET_SECONDS`. An ordinary exact test command can omit a custom lane result: RMS wraps a successful nonzero selected-test execution in `rms/hunt-lane-result/v0.1`, and it converts a failed test execution into a finding. A zero-test or uncountable test execution is invalid even when the command exits zero. Other runners must write `rms/hunt-lane-result/v0.1` to the exact path in `RMS_HUNT_OUTPUT`. A custom result can add richer metrics, findings, and artifacts. A minimal passing result is:

   ```yaml
   spec: rms/hunt-lane-result/v0.1
   status: pass
   metrics:
     cases: 1024
   artifacts: []
   ```

   Valid statuses are `pass`, `finding`, `inconclusive`, `invalid`, and `unsupported`. A finding also includes `findings[]` entries with `kind`, nonblank `summary`, and replay/artifact references when available. Record relevant cases, coverage, mutants, findings, and artifacts; do not write lane YAML to stdout in place of the requested file.
6. Report a behavioral failure only with a minimized replayable counterexample. Preserve and surface each serious behavioral finding immediately. Continue other distinct lanes within the recorded budget; do not stop the complete campaign after the first finding. RMS deduplicates findings by stable minimized replay signature and retains the shortest replay. Use `rms property replay <analysis>` for property findings and the recorded replay recipe for tool-native counterexamples. A custom replay command exits zero only when it reproduces; RMS-native replay JSON with `result: reproduced|replayed` is also accepted.
7. Resume interrupted work with `rms hunt --root <root> --resume latest`. Do not resume after source, declaration, tool, or seed drift.
8. Read `rms/hunt-report/v0.2` or the RMS viewer; v0.1 remains readable. Triage stable finding IDs and their shortest replay first, and distinguish latest, recurring, and not-observed findings. Distinguish `bugs-found`, `proof-gaps-found`, `clean-under-recorded-bounds`, `inconclusive`, `invalid`, and `unsupported`. Guided-search exhaustion remains bounded evidence and never enters finite proof scope.
9. Promote retained counterexamples into smoke regressions. Fix invalid declarations or proof gaps before claiming the hunt is useful.
10. Finish software changes through focused proof, `rms check --changes`, an authorized candidate commit, and `rms check --committed`. An overnight hunt complements these gates; it does not replace them.

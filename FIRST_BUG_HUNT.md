# Your First RMS Bug Hunt

This tutorial has two parts:

1. Run a five-minute demonstration that already exists in this repository.
2. Apply the same method to a small visual Snake game without giving the agent the answer.

The first part teaches the mechanics. The second tests whether RMS guidance transfers to new software.

## What a Hunt Can Claim

RMS separates proof from bounded evidence:

- Exhausted finite exploration proves only the declared finite model.
- Fuzzing, generated cases, sanitizers, mutation testing, and guided search provide bounded evidence.
- A replayable behavioral failure is a product bug only when the checked expectation is a genuine product promise.
- A completed hunt never means that arbitrary software is bug-free.

Every retained behavioral finding must include a counterexample that reproduces.

## Part 1: Run the Teaching Hunt

### Preconditions

Use a clean committed checkout and an RMS binary built from that checkout:

```bash
rms --version
git status --short
```

`git status --short` must print nothing. Hunt execution uses the exact commit in an isolated checkout; it refuses an uncommitted source tree so a result can be reproduced later.

### Inspect the campaign

```bash
rms hunt \
  --root . \
  --assembly examples/probes/public-rust-workload-failures.yaml \
  --budget 30s \
  --seed 7 \
  --dry-run
```

The plan should contain one `guided-semantic-novelty-v1` probe-exploration lane. A dry run executes no project code.

### Find the demonstrations

```bash
rms hunt \
  --root . \
  --assembly examples/probes/public-rust-workload-failures.yaml \
  --budget 30s \
  --seed 7 \
  --out .rms/first-bug-hunt.yaml
```

The committed teaching assembly deliberately makes two incompatible claims about the same public workload: every input must describe three widgets, and every input must reject three widgets. The expected result is therefore `bugs-found` with two distinct findings. These findings demonstrate discovery and replay; they are not claims that the Rust example violates its real product contract.

The report records:

- the exact source revision, RMS binary identity, seed, and budget;
- the executed lane and its bounded search metrics;
- stable finding IDs and occurrence counts;
- the failed check and first bad transition;
- minimized counterexample paths and copyable replay recipes;
- an explicit statement that guided exploration did not prove the full model.

### Replay one finding

Open `.rms/first-bug-hunt.yaml`, copy either finding's `replay` command, and run it. A reproduced probe counterexample exits `1`; that exit means “the recorded failure reproduced,” not “the replay tool malfunctioned.” Exit `0` means the recorded failure is resolved, and exit `2` means the artifact is invalid or no longer executable.

The teaching loop is now complete:

```text
promise → exploration → minimized finding → replay → honest proof scope
```

## Part 2: Transfer the Method to Snake

Snake is a useful first project because its rules are visible, its core is deterministic, and its interesting failures are easy to understand. Use a 6×6 board, one apple, fixed ticks, arrow-key input, no wrapping, and a thin browser interface over a pure game machine.

### Give a fresh agent this brief

Start in a new directory and a new agent task with the installed RMS plugin active. Give the agent only this request:

> Build a small, polished 6×6 web Snake game using the installed RMS guidance. Use arrow keys, one apple, fixed deterministic ticks, no wrapping, score on eating, collision-based game over, and winning by filling the board. Keep the browser as a thin visual boundary over deterministic game rules. Make the project suitable for unattended bug hunting. Finish with a clean committed candidate, but do not claim the game is bug-free merely because tests pass.

Do not prescribe the module topology, implementation language, property syntax, or proof commands. The point is to observe whether the agent discovers and follows the installed RMS workflow.

### Expected promises

The resulting semantics should make these rules executable rather than leaving them in prose:

| Promise | Appropriate evidence |
| --- | --- |
| A living snake occupies unique cells | Generated property and finite exploration |
| The living head remains inside the board | Generated property and finite exploration |
| Food never occupies the snake | Generated property over placement decisions |
| Eating increases length and score exactly once | Transition property and replay trace |
| A move without food preserves length and score | Transition property |
| Immediate reversal is rejected or ignored consistently | Exhaustive direction cases |
| Wall or self-collision ends the game | Finite state exploration |
| Ticks cannot change a finished game | State-machine property |
| The same state and input produce the same result | Determinism property |
| Keyboard and browser input cannot bypass parsing | Boundary fuzzing |

Use a smaller board, such as 4×4, for exhaustive exploration when the complete 6×6 state space is impractical. The proof claim must name the explored model rather than silently generalizing it to every board.

### Inspect what the agent prepared

From the clean committed Snake project:

```bash
rms check --committed --root .
rms hunt --root . --dry-run
```

The dry run should show risk-derived strong lanes or focused exceptions. A credible project normally includes pure/generated properties, finite machine exploration, browser-input fuzzing, and automatic replay of historical counterexamples. Missing tools should appear as `unsupported`; missing proof strength should appear as a proof gap, not disappear.

### Run a short hunt

```bash
rms hunt \
  --root . \
  --budget 10m \
  --seed 7 \
  --out .rms/snake-hunt.yaml
```

The result may honestly be `bugs-found`, `proof-gaps-found`, `clean-under-recorded-bounds`, `inconclusive`, or `unsupported`. Judge the report by whether it names what ran, what did not run, what was exhausted, and how each behavioral finding replays—not by whether the headline is green.

### Optional blind bug exercise

To test discovery rather than construction, have someone other than the hunting agent introduce and commit one small defect after the correct baseline exists. Good seeds include:

- leaving the game active after self-collision;
- allowing food to be placed inside the snake;
- incrementing score twice for one apple;
- accepting an immediate reverse direction;
- allowing one final tick to mutate a finished game.

Do not tell the hunting agent which defect was introduced. Ask a fresh task:

> Use the installed RMS guidance to hunt for bugs in this committed project. Report only replayable behavioral findings, explain the smallest failing story in game terms, and state the exact proof scope.

A successful hunt should reduce the failure to a short sequence such as:

```text
state: snake occupies (2,2), (2,3), (1,3); direction left
inputs: down → right → up
observed: head enters (2,2), but status remains playing
violated: a living snake occupies unique cells
```

Fix the underlying rule, replay the counterexample until it reports resolved, then promote the counterexample into the smoke regression corpus.

## Dogfood Scorecard

The experiment succeeds when a context-free agent:

- starts from RMS guidance and records real product promises;
- keeps deterministic rules separate from the browser boundary;
- derives strong lanes from declared risk;
- runs expensive exploration separately from fast commit checks;
- retains only replayable behavioral findings;
- explains failures in visible game terms;
- distinguishes finite proof, bounded evidence, proof gaps, and unsupported work;
- completes against a clean committed candidate.

The experiment fails if the agent merely adds conventional unit tests, treats a fixed input list as fuzzing, reports an unreplayable failure, or translates “nothing failed during the budget” into “bug-free.”

## Continue Exploring

Use `rms view --root .` to inspect live and completed hunt reports. Use `rms hunt --root . --resume latest` to continue an interrupted campaign without changing its recorded revision, declarations, tools, seed, or configuration.

For exact command and artifact semantics, see the [RMS Reference](REFERENCE.md#proof-first-bug-hunt).

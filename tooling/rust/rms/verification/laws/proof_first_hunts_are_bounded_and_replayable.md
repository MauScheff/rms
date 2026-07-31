# Proof-First Hunts Are Bounded and Replayable

Risk determines which strong lane must exist; it does not turn every lane into the same kind of claim.

- Exhausted finite exploration proves only its declared model.
- Fuzzing, generated cases, sanitizers, and mutation testing are bounded evidence.
- Seeded semantic-novelty exploration is bounded evidence even when its frontier is exhausted.
- Surviving mutants and inadequate coverage are proof gaps.
- Behavioral failures are findings only after a minimized counterexample reaches its exact target failure on replay; discovery continues to a fixed cap of distinct semantic findings.
- A resumed run must retain source revision, declaration digest, tool identities, and seed.
- The caller's tracked checkout is never the workspace of an unattended lane.

Evidence is the `hunt-rms-system` contract suite, risk-posture property test, report-schema and stable-finding tests, guided multi-finding replay test, isolated CLI smoke, and the retained `rms/hunt-report/v0.2`; v0.1 remains readable.

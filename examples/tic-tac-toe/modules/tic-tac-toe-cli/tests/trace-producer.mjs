import { writeFile } from "node:fs/promises";
import {
  awaitingInput,
  commandInput,
  effectResultInput,
  rulesBridgeFailed,
  rulesBridgeSucceeded,
  runMove,
} from "../src/representation.mjs";
import { transitionRecord } from "../src/transition.mjs";

export async function produceTransitionTrace() {
  const output = process.env.RMS_TRACE_OUTPUT;
  if (!output) throw new Error("RMS_TRACE_OUTPUT is required");

  const requested = transitionRecord(awaitingInput(), commandInput(runMove("A1")));
  const succeeded = transitionRecord(
    requested.state_after,
    effectResultInput(rulesBridgeSucceeded({ outcome: { tag: "Accepted" } })),
  );
  const rejected = transitionRecord(awaitingInput(), commandInput(runMove("")));
  const failed = transitionRecord(
    requested.state_after,
    effectResultInput(rulesBridgeFailed("bridge unavailable")),
  );
  const records = [
    { scenario_start: true, ...requested },
    { scenario_start: false, ...succeeded },
    { scenario_start: true, ...rejected },
    { scenario_start: true, ...failed },
  ];

  await writeFile(output, JSON.stringify({
    spec: "rms/trace-bundle/v0.1",
    machine: "TicTacToeBoundaryMachine",
    records,
  }, null, 2));
  if (records.length !== 4) throw new Error("incomplete boundary trace");
}

if (process.env.RMS_TRACE_RUNNER?.endsWith("#produceTransitionTrace")) {
  await produceTransitionTrace();
}

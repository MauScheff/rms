import assert from "node:assert/strict";
import { createBoundarySession, handleBoundaryInput } from "../src/adapter.mjs";
import { generateMalformedInputCases } from "../src/parser.mjs";
import { createRulesPort } from "../src/ports.mjs";

export function runMalformedInputProperty() {
  const delegated = [];
  const rulesPort = createRulesPort({
    applyMove: (move) => {
      delegated.push(move);
      return Object.freeze({ outcome: Object.freeze({ tag: "Accepted" }), move });
    },
  });
  const session = createBoundarySession(rulesPort);

  for (const input of generateMalformedInputCases()) {
    assert.equal(handleBoundaryInput(input).tag, "Rejected");
    assert.equal(session.handle(input).tag, "Rejected");
  }
  assert.deepEqual(delegated, []);

  assert.equal(session.handle("A1").tag, "Accepted");
  assert.equal(session.handle("b2").tag, "Accepted");
  assert.deepEqual(
    delegated.map((move) => move.cell.index),
    [0, 4],
  );
  assert.deepEqual(session.state().acceptedMoveIndexes, [0, 4]);
}

const requestedRunner = process.env.RMS_PROPERTY_RUNNER;
if (requestedRunner && !requestedRunner.endsWith("#runMalformedInputProperty")) {
  throw new Error(`unsupported RMS property runner: ${requestedRunner}`);
}
runMalformedInputProperty();

import assert from "node:assert/strict";
import { createBoundarySession } from "../src/adapter.mjs";
import { createRulesPort } from "../src/ports.mjs";

const delegated = [];
const rulesPort = createRulesPort({
  applyMove: (move) => {
    delegated.push(move);
    return Object.freeze({ outcome: Object.freeze({ tag: "Accepted" }), move });
  },
});
const session = createBoundarySession(rulesPort);

assert.equal(session.handle("").tag, "Rejected");
assert.equal(session.handle("D4").tag, "Rejected");
assert.equal(session.handle("A1").tag, "Accepted");
assert.equal(session.handle("b2").tag, "Accepted");
assert.deepEqual(
  delegated.map((move) => move.cell.index),
  [0, 4],
);
assert.deepEqual(session.state().acceptedMoveIndexes, [0, 4]);

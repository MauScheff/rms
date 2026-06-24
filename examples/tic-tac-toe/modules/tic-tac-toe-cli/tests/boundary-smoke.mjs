import assert from "node:assert/strict";
import { handleBoundaryInput } from "../src/adapter.mjs";
import { createRulesPort } from "../src/ports.mjs";

const delegated = [];
const rulesPort = createRulesPort({
  applyMove: (move) => {
    delegated.push(move);
    return Object.freeze({ tag: "RulesAccepted", move });
  },
});

assert.equal(handleBoundaryInput("", rulesPort).tag, "Rejected");
assert.equal(handleBoundaryInput("D4", rulesPort).tag, "Rejected");
assert.equal(handleBoundaryInput("A1", rulesPort).tag, "Accepted");
assert.equal(handleBoundaryInput("b2", rulesPort).tag, "Accepted");
assert.deepEqual(
  delegated.map((move) => move.cell.index),
  [0, 4],
);

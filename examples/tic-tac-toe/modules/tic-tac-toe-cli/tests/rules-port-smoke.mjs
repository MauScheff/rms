import assert from "node:assert/strict";
import { handleBoundaryInput } from "../src/adapter.mjs";
import { createRulesPort } from "../src/ports.mjs";

const rulesPort = createRulesPort();

const first = handleBoundaryInput("A1", rulesPort);
assert.equal(first.tag, "Accepted");
assert.equal(first.value.outcome.tag, "Accepted");
assert.equal(first.value.state.board[0], "X");
assert.equal(first.value.state.status.next, "O");

const occupied = handleBoundaryInput("A1", rulesPort);
assert.equal(occupied.tag, "Accepted");
assert.equal(occupied.value.outcome.tag, "Rejected");
assert.equal(occupied.value.outcome.reason, "CellOccupied");

const second = handleBoundaryInput("B2", rulesPort);
assert.equal(second.tag, "Accepted");
assert.equal(second.value.outcome.tag, "Accepted");
assert.equal(second.value.state.board[4], "O");

assert.deepEqual(rulesPort.acceptedMoveIndexes(), [0, 4]);

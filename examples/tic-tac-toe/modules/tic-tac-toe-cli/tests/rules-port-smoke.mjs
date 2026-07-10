import assert from "node:assert/strict";
import { createBoundarySession } from "../src/adapter.mjs";
import { createRulesPort } from "../src/ports.mjs";

const rulesPort = createRulesPort();
const session = createBoundarySession(rulesPort);

const first = session.handle("A1");
assert.equal(first.tag, "Accepted");
assert.equal(first.value.outcome.tag, "Accepted");
assert.equal(first.value.state.board[0], "X");
assert.equal(first.value.state.status.next, "O");

const occupied = session.handle("A1");
assert.equal(occupied.tag, "Accepted");
assert.equal(occupied.value.outcome.tag, "Rejected");
assert.equal(occupied.value.outcome.reason, "CellOccupied");

const second = session.handle("B2");
assert.equal(second.tag, "Accepted");
assert.equal(second.value.outcome.tag, "Accepted");
assert.equal(second.value.state.board[4], "O");

assert.deepEqual(session.state().acceptedMoveIndexes, [0, 4]);

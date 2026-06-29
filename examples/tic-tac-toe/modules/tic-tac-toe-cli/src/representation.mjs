export function makeCell(row, column) {
  if (!Number.isInteger(row) || !Number.isInteger(column)) {
    return null;
  }
  if (row < 0 || row > 2 || column < 0 || column > 2) {
    return null;
  }
  return Object.freeze({ tag: "Cell", row, column, index: row * 3 + column });
}

export const BoundaryState = Object.freeze({
  Ready: "BoundaryState.Ready",
});

export function makeRawCliInput(text) {
  return Object.freeze({ tag: "RawCliInput", text });
}

export function parsedMove(cell) {
  return Object.freeze({ tag: "ParsedMove", cell });
}

export const BoundaryResult = Object.freeze({
  Accepted: "Accepted",
  Rejected: "Rejected",
});

export function accepted(value) {
  return Object.freeze({ tag: "Accepted", value });
}

export function rejected(reason) {
  return Object.freeze({ tag: "Rejected", reason });
}

export function makeInvokeRulesBridge(move) {
  return Object.freeze({ tag: "InvokeRulesBridge", move });
}

export function makeRulesBridgeResult(result) {
  return Object.freeze({ tag: "RulesBridgeResult", result });
}

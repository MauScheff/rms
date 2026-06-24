export function makeCell(row, column) {
  if (!Number.isInteger(row) || !Number.isInteger(column)) {
    return null;
  }
  if (row < 0 || row > 2 || column < 0 || column > 2) {
    return null;
  }
  return Object.freeze({ tag: "Cell", row, column, index: row * 3 + column });
}

export function parsedMove(cell) {
  return Object.freeze({ tag: "ParsedMove", cell });
}

export function accepted(value) {
  return Object.freeze({ tag: "Accepted", value });
}

export function rejected(reason) {
  return Object.freeze({ tag: "Rejected", reason });
}

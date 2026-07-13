import {
  makeCell,
  parsedMove,
  rejected,
  TicTacToeBoundaryRejection,
} from "./representation.mjs";

const LETTER_ROWS = new Map([
  ["a", 0],
  ["b", 1],
  ["c", 2],
]);

export function generateMalformedInputCases() {
  const whitespace = ["", " ", "\n", "\t", " \n\t "];
  const outOfBoard = ["D4", "A0", "4,4", "-1,1"];
  const incomplete = ["A", "1", "1,", ",1", "A1B2"];
  const nonText = [null, undefined, Object.freeze({ row: 1, column: 1 })];

  return Object.freeze([...whitespace, ...outOfBoard, ...incomplete, ...nonText]);
}

export function parseMoveText(input) {
  if (typeof input !== "string") {
    return rejected(TicTacToeBoundaryRejection.MalformedInput);
  }

  const value = input.trim().toLowerCase();
  if (!value) {
    return rejected(TicTacToeBoundaryRejection.MalformedInput);
  }

  const compact = value.replace(/\s+/g, "");
  const letterMatch = compact.match(/^([abc])([123])$/);
  if (letterMatch) {
    const row = LETTER_ROWS.get(letterMatch[1]);
    const column = Number(letterMatch[2]) - 1;
    const cell = makeCell(row, column);
    return cell ? parsedMove(cell) : rejected(TicTacToeBoundaryRejection.OutOfBoard);
  }

  const numericMatch = compact.match(/^([1-3]),?([1-3])$/);
  if (numericMatch) {
    const row = Number(numericMatch[1]) - 1;
    const column = Number(numericMatch[2]) - 1;
    const cell = makeCell(row, column);
    return cell ? parsedMove(cell) : rejected(TicTacToeBoundaryRejection.OutOfBoard);
  }

  return rejected(TicTacToeBoundaryRejection.MalformedInput);
}

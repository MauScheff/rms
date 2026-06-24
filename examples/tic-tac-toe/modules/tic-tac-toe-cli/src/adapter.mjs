import { parseMoveText } from "./parser.mjs";
import { accepted } from "./representation.mjs";

export function handleBoundaryInput(input, rulesPort) {
  const parsed = parseMoveText(input);
  if (parsed.tag === "Rejected") {
    return parsed;
  }
  return accepted(rulesPort.applyMove(parsed));
}

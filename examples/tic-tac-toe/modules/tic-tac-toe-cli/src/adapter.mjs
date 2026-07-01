import { parseMoveText } from "./parser.mjs";
import {
  AwaitingInput,
  Completed,
  Rejected,
  accepted,
} from "./representation.mjs";

function transitionOutput(nextState, events, reply) {
  return Object.freeze({
    tag: "BoundaryTransition",
    next_state: nextState,
    events: Object.freeze(events),
    commands: Object.freeze([]),
    effects: Object.freeze([]),
    reply,
  });
}

export function handleBoundaryTransition(input, rulesPort) {
  const stateBefore = AwaitingInput;
  const parsed = parseMoveText(input);
  if (parsed.tag === "Rejected") {
    const output = transitionOutput(Rejected, [parsed], parsed);
    return Object.freeze({
      tag: "BoundaryTransitionRecord",
      state_before: stateBefore,
      state_after: Rejected,
      input,
      output,
      source: Object.freeze({
        file: "src/adapter.mjs",
        function: "handleBoundaryTransition",
        branch: "Rejected",
      }),
    });
  }
  const reply = accepted(rulesPort.applyMove(parsed));
  const output = transitionOutput(Completed, [parsed], reply);
  return Object.freeze({
    tag: "BoundaryTransitionRecord",
    state_before: stateBefore,
    state_after: Completed,
    input: parsed,
    output,
    source: Object.freeze({
      file: "src/adapter.mjs",
      function: "handleBoundaryTransition",
      branch: "ParsedMove",
    }),
  });
}

export function handleBoundaryInput(input, rulesPort) {
  return handleBoundaryTransition(input, rulesPort).output.reply;
}

export const TicTacToeCliBoundary = Object.freeze({
  handleBoundaryTransition,
  handleBoundaryInput,
});

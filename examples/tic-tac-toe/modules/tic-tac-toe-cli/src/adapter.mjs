import {
  awaitingInput,
  commandInput,
  effectResultInput,
  runMove,
} from "./representation.mjs";
import { transitionRecord } from "./transition.mjs";

export function createBoundarySession(effectExecutor) {
  let state = awaitingInput();
  return Object.freeze({
    handle(inputText) {
      const commandRecord = transitionRecord(state, commandInput(runMove(inputText)));
      state = commandRecord.state_after;
      const effect = commandRecord.output.effects[0];
      if (!effect) return commandRecord.output.reply;

      const resultRecord = transitionRecord(
        state,
        effectResultInput(effectExecutor.execute(effect)),
      );
      state = resultRecord.state_after;
      return resultRecord.output.reply;
    },
    state() {
      return state;
    },
  });
}

export function handleBoundaryInput(inputText, effectExecutor) {
  return createBoundarySession(effectExecutor).handle(inputText);
}

export const TicTacToeBoundaryAdapter = Object.freeze({
  createBoundarySession,
  handleBoundaryInput,
});

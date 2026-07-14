import {
  awaitingInput,
  commandInput,
  effectResultInput,
  rejected,
  runMove,
} from "./representation.mjs";
import { driveMachine } from "./machine_driver.mjs";
import { transitionRecord } from "./transition.mjs";

function boundaryResult(output) {
  return output.reply ?? (output.rejection ? rejected(output.rejection) : null);
}

export function createBoundarySession(effectExecutor) {
  let state = awaitingInput();
  return Object.freeze({
    handle(inputText) {
      const commandRecord = transitionRecord(state, commandInput(runMove(inputText)));
      state = commandRecord.state_after;
      const effect = commandRecord.output.effects[0];
      if (!effect) return boundaryResult(commandRecord.output);

      const resultRecord = transitionRecord(
        state,
        effectResultInput(effectExecutor.execute(effect)),
      );
      state = resultRecord.state_after;
      return boundaryResult(resultRecord.output);
    },
    state() {
      return state;
    },
  });
}

export function handleBoundaryInput(inputText) {
  const records = driveMachine(awaitingInput(), commandInput(runMove(inputText)));
  return boundaryResult(records.at(-1)?.output ?? {});
}

export const TicTacToeBoundaryAdapter = Object.freeze({
  createBoundarySession,
  handleBoundaryInput,
});

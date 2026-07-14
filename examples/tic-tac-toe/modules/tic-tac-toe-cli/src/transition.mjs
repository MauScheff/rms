import { parseMoveText } from "./parser.mjs";
import {
  accepted,
  awaitingInput,
  awaitingRulesResult,
  invokeRulesBridge,
  moveParsed,
  rulesInvocationCompleted,
  rulesInvocationRequested,
  runRejected,
  TicTacToeBoundaryRejection,
  transitionOutput,
  transitionRecord as makeTransitionRecord,
} from "./representation.mjs";

export function transition(state, input) {
  return transitionRecord(state, input).output;
}

export function transitionRecord(state, input) {
  if (state.tag === "AwaitingInput" && input.tag === "Command") {
    const parsed = parseMoveText(input.command.text);
    if (parsed.tag === "Rejected") {
      const output = transitionOutput(
        state,
        [runRejected(parsed.reason)],
        [],
        null,
        parsed.reason,
      );
      return makeTransitionRecord(state, input, output, "RunRejected");
    }
    const effect = invokeRulesBridge(state.acceptedMoveIndexes, parsed);
    const output = transitionOutput(
      awaitingRulesResult(state.acceptedMoveIndexes, parsed),
      [moveParsed(parsed), rulesInvocationRequested()],
      [effect],
      null,
    );
    return makeTransitionRecord(state, input, output, "RunMove");
  }

  if (state.tag === "AwaitingRulesResult" && input.tag === "EffectResult") {
    if (input.effectResult.tag === "RulesBridgeSucceeded") {
      const result = input.effectResult.result;
      const acceptedMoveIndexes = result.outcome?.tag === "Accepted"
        ? [...state.acceptedMoveIndexes, state.move.cell.index]
        : state.acceptedMoveIndexes;
      const output = transitionOutput(
        awaitingInput(acceptedMoveIndexes),
        [rulesInvocationCompleted()],
        [],
        accepted(result),
      );
      return makeTransitionRecord(state, input, output, "RulesBridgeSucceeded");
    }
    if (input.effectResult.tag === "RulesBridgeFailed") {
      const output = transitionOutput(
        awaitingInput(state.acceptedMoveIndexes),
        [runRejected(TicTacToeBoundaryRejection.RulesBridgeFailure)],
        [],
        null,
        TicTacToeBoundaryRejection.RulesBridgeFailure,
      );
      return makeTransitionRecord(state, input, output, "RulesBridgeFailed");
    }
  }

  const output = transitionOutput(
    state,
    [runRejected(TicTacToeBoundaryRejection.IllegalTransition)],
    [],
    null,
    TicTacToeBoundaryRejection.IllegalTransition,
  );
  return makeTransitionRecord(state, input, output, "IllegalTransition");
}

export function replayTrace(initialState, inputs) {
  const records = [];
  let state = initialState;
  for (const input of inputs) {
    const record = transitionRecord(state, input);
    records.push(record);
    state = record.state_after;
  }
  return Object.freeze(records);
}

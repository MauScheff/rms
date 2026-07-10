export function makeCell(row, column) {
  if (!Number.isInteger(row) || !Number.isInteger(column)) return null;
  if (row < 0 || row > 2 || column < 0 || column > 2) return null;
  return Object.freeze({ tag: "Cell", row, column, index: row * 3 + column });
}

export function awaitingInput(acceptedMoveIndexes = []) {
  return Object.freeze({
    tag: "AwaitingInput",
    acceptedMoveIndexes: Object.freeze([...acceptedMoveIndexes]),
  });
}

export function awaitingRulesResult(acceptedMoveIndexes, move) {
  return Object.freeze({
    tag: "AwaitingRulesResult",
    acceptedMoveIndexes: Object.freeze([...acceptedMoveIndexes]),
    move,
  });
}

export const TicTacToeBoundaryState = Object.freeze({
  AwaitingInput: awaitingInput,
  AwaitingRulesResult: awaitingRulesResult,
});

export function runMove(text) {
  return Object.freeze({ tag: "RunMove", text });
}

export const TicTacToeBoundaryCommand = Object.freeze({ RunMove: runMove });

export function commandInput(command) {
  return Object.freeze({ tag: "Command", command });
}

export function effectResultInput(effectResult) {
  return Object.freeze({ tag: "EffectResult", effectResult });
}

export const TicTacToeBoundaryInput = Object.freeze({
  Command: commandInput,
  EffectResult: effectResultInput,
});

export function parsedMove(cell) {
  return Object.freeze({ tag: "ParsedMove", cell });
}

export function moveParsed(move) {
  return Object.freeze({ tag: "MoveParsed", move });
}

export function rulesInvocationRequested() {
  return Object.freeze({ tag: "RulesInvocationRequested" });
}

export function rulesInvocationCompleted() {
  return Object.freeze({ tag: "RulesInvocationCompleted" });
}

export function runRejected(reason) {
  return Object.freeze({ tag: "RunRejected", reason });
}

export const TicTacToeBoundaryEvent = Object.freeze({
  MoveParsed: moveParsed,
  RulesInvocationRequested: rulesInvocationRequested,
  RulesInvocationCompleted: rulesInvocationCompleted,
  RunRejected: runRejected,
});

export function invokeRulesBridge(acceptedMoveIndexes, move) {
  return Object.freeze({
    tag: "InvokeRulesBridge",
    acceptedMoveIndexes: Object.freeze([...acceptedMoveIndexes]),
    move,
  });
}

export const TicTacToeBoundaryEffect = Object.freeze({
  InvokeRulesBridge: invokeRulesBridge,
});

export function rulesBridgeSucceeded(result) {
  return Object.freeze({ tag: "RulesBridgeSucceeded", result });
}

export function rulesBridgeFailed(reason) {
  return Object.freeze({ tag: "RulesBridgeFailed", reason });
}

export const TicTacToeBoundaryEffectResult = Object.freeze({
  RulesBridgeSucceeded: rulesBridgeSucceeded,
  RulesBridgeFailed: rulesBridgeFailed,
});

export function accepted(value) {
  return Object.freeze({ tag: "Accepted", value });
}

export function rejected(reason) {
  return Object.freeze({ tag: "Rejected", reason });
}

export const TicTacToeBoundaryReply = Object.freeze({ Accepted: accepted, Rejected: rejected });

export const TicTacToeBoundaryRejection = Object.freeze({
  MalformedInput: "MalformedInput",
  OutOfBoard: "OutOfBoard",
  RulesBridgeFailure: "RulesBridgeFailure",
  IllegalTransition: "IllegalTransition",
});

export function transitionOutput(nextState, events, effects, reply) {
  return Object.freeze({
    tag: "TicTacToeBoundaryTransition",
    next_state: nextState,
    events: Object.freeze(events),
    commands: Object.freeze([]),
    effects: Object.freeze(effects),
    reply,
  });
}

export const TicTacToeBoundaryTransition = Object.freeze({ create: transitionOutput });

export function transitionRecord(stateBefore, input, output, branch) {
  return Object.freeze({
    tag: "TicTacToeBoundaryTransitionRecord",
    state_before: stateBefore,
    state_after: output.next_state,
    input,
    output,
    source: Object.freeze({ file: "src/transition.mjs", function: "transition", branch }),
  });
}

export const TicTacToeBoundaryTransitionRecord = Object.freeze({ create: transitionRecord });

export const TicTacToeBoundaryMachine = Object.freeze({ name: "TicTacToeBoundaryMachine" });

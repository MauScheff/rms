import { executeRulesBridge } from "./ports.mjs";
import { effectResultInput } from "./representation.mjs";
import { transitionRecord } from "./transition.mjs";

export function driveMachine(initialState, initialInput) {
  let state = initialState;
  const pending = [initialInput];
  const records = [];
  while (pending.length > 0) {
    const input = pending.pop();
    const record = transitionRecord(state, input);
    state = record.state_after;
    for (const effect of [...record.output.effects].reverse()) {
      pending.push(effectResultInput(executeRulesBridge(effect)));
    }
    records.push(record);
  }
  return Object.freeze(records);
}

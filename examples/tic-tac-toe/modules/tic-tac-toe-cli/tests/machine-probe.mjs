import { readFile, writeFile } from "node:fs/promises";
import {
  awaitingInput,
  awaitingRulesResult,
  commandInput,
  effectResultInput,
  initialState,
  rulesBridgeFailed,
  rulesBridgeSucceeded,
  runMove,
} from "../src/representation.mjs";
import { transitionRecord } from "../src/transition.mjs";

function stateJSON(state) {
  if (state.tag === "AwaitingRulesResult") {
    return {
      name: state.tag,
      data: {
        accepted_move_indexes: state.acceptedMoveIndexes,
        move: {
          row: state.move.cell.row,
          column: state.move.cell.column,
        },
      },
    };
  }
  return {
    name: state.tag,
    data: { accepted_move_indexes: state.acceptedMoveIndexes },
  };
}

function parseState(value) {
  const accepted = value?.data?.accepted_move_indexes ?? [];
  if (value?.name === "AwaitingRulesResult") {
    const { row, column } = value.data.move;
    const index = row * 3 + column;
    return awaitingRulesResult(accepted, {
      tag: "ParsedMove",
      cell: { tag: "Cell", row, column, index },
    });
  }
  return awaitingInput(accepted);
}

function parseInput(value) {
  if (value.kind === "command" && value.name === "RunMove") {
    return commandInput(runMove(value.data.text));
  }
  if (value.kind === "effect-result" && value.name === "RulesBridgeSucceeded") {
    return effectResultInput(rulesBridgeSucceeded(value.data.result ?? {
      outcome: { tag: "Accepted" },
    }));
  }
  if (value.kind === "effect-result" && value.name === "RulesBridgeFailed") {
    return effectResultInput(rulesBridgeFailed(value.data.reason ?? "probe failure"));
  }
  throw new Error(`unsupported probe input ${value.kind}:${value.name}`);
}

function named(value) {
  if (value == null) return null;
  if (typeof value === "string") return { name: value, data: {} };
  const { tag, ...data } = value;
  return { name: tag, data };
}

function recordJSON(record, input, scenarioStart) {
  return {
    scenario_start: scenarioStart,
    state_before: stateJSON(record.state_before),
    state_after: stateJSON(record.state_after),
    input,
    output: {
      next_state: stateJSON(record.output.next_state),
      events: record.output.events.map(named),
      commands: record.output.commands.map(named),
      effects: record.output.effects.map(named),
      reply: named(record.output.reply),
      rejection: named(record.output.rejection),
    },
    source: record.source,
  };
}

export async function probeMachine() {
  const requestPath = process.env.RMS_PROBE_REQUEST;
  const outputPath = process.env.RMS_PROBE_OUTPUT;
  if (!requestPath || !outputPath) {
    throw new Error("RMS_PROBE_REQUEST and RMS_PROBE_OUTPUT are required");
  }
  const request = JSON.parse(await readFile(requestPath, "utf8"));
  let output;
  if (request.operation === "describe") {
    output = {
      spec: "rms/machine-probe-description/v0.1",
      machine: "TicTacToeBoundaryMachine",
      initial_state: stateJSON(initialState()),
      states: [
        {
          name: "AwaitingInput",
          data_schema: { type: "object" },
          examples: [stateJSON(initialState())],
        },
        {
          name: "AwaitingRulesResult",
          data_schema: { type: "object" },
          examples: [{
            name: "AwaitingRulesResult",
            data: {
              accepted_move_indexes: [],
              move: { row: 0, column: 0 },
            },
          }],
        },
      ],
      inputs: [
        {
          kind: "command",
          name: "RunMove",
          data_schema: {
            type: "object",
            properties: { text: { type: "string" } },
            required: ["text"],
          },
          example: { kind: "command", name: "RunMove", data: { text: "A1" } },
        },
        {
          kind: "effect-result",
          name: "RulesBridgeSucceeded",
          data_schema: { type: "object" },
          example: {
            kind: "effect-result",
            name: "RulesBridgeSucceeded",
            data: { result: { outcome: { tag: "Accepted" } } },
          },
        },
        {
          kind: "effect-result",
          name: "RulesBridgeFailed",
          data_schema: {
            type: "object",
            properties: { reason: { type: "string" } },
          },
          example: {
            kind: "effect-result",
            name: "RulesBridgeFailed",
            data: { reason: "bridge failed" },
          },
        },
      ],
    };
  } else if (request.operation === "evaluate") {
    output = {
      spec: "rms/machine-probe-evaluation/v0.2",
      machine: "TicTacToeBoundaryMachine",
      results: request.cases.map((item) => {
        const record = transitionRecord(parseState(item.state), parseInput(item.input));
        return {
          id: item.id,
          record: recordJSON(record, item.input, true),
        };
      }),
    };
  } else {
    let state = request.start === "initial" ? initialState() : parseState(request.start);
    const records = request.steps.map((step, index) => {
      const record = transitionRecord(state, parseInput(step.input));
      state = record.state_after;
      return recordJSON(record, step.input, index === 0);
    });
    output = {
      spec: "rms/trace-bundle/v0.1",
      machine: "TicTacToeBoundaryMachine",
      records,
    };
  }
  await writeFile(outputPath, JSON.stringify(output, null, 2));
}

if (process.env.RMS_PROBE_RUNNER?.split("#").at(-1) === "probeMachine") {
  await probeMachine();
}

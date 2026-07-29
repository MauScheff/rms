import Foundation
import XCTest
@testable import RmsSwiftExample

private enum ProbeFailure: Error {
    case message(String)
}

final class MachineProbeTests: XCTestCase {
    private func named(_ name: String, _ data: [String: Any] = [:]) -> [String: Any] {
        ["name": name, "data": data]
    }

    private func parseInput(_ value: [String: Any]) throws -> DescribeSwiftWidgetCommand {
        let name = value["name"] as? String
        let data = value["data"] as? [String: Any] ?? [:]
        switch name {
        case "Describe":
            guard
                let raw = data["name"] as? String,
                let widget = SwiftWidget(raw)
            else { throw ProbeFailure.message("Describe.data.name must be non-empty") }
            return .describe(widget)
        case "RejectEmptyName":
            return .rejectEmptyName(data["name"] as? String ?? "")
        default:
            throw ProbeFailure.message("unsupported probe command \(name ?? "<missing>")")
        }
    }

    private func recordJSON(
        _ record: SwiftExampleTransitionRecord,
        _ input: [String: Any],
        scenarioStart: Bool
    ) -> [String: Any] {
        let events = record.output.events.map { event in
            switch event {
            case .widgetDescribed: return named("WidgetDescribed")
            case .widgetRejected: return named("WidgetRejected")
            }
        }
        let reply: Any = record.output.reply.map { reply in
            switch reply {
            case .description(let value): return named("Description", ["value": value])
            }
        } ?? NSNull()
        let rejection: Any = record.output.rejection.map { rejection in
            switch rejection {
            case .emptyWidgetName: return named("EmptyWidgetName")
            case .expectedEmptyName: return named("ExpectedEmptyName")
            }
        } ?? NSNull()
        return [
            "scenario_start": scenarioStart,
            "state_before": named("Ready"),
            "state_after": named("Ready"),
            "input": input,
            "output": [
                "next_state": named("Ready"),
                "events": events,
                "commands": [],
                "effects": [],
                "reply": reply,
                "rejection": rejection,
            ],
            "source": [
                "file": record.source.file,
                "function": record.source.function,
                "branch": record.source.branch,
            ],
        ]
    }

    func testProbeMachine() throws {
        let environment = ProcessInfo.processInfo.environment
        guard
            let requestPath = environment["RMS_PROBE_REQUEST"],
            let outputPath = environment["RMS_PROBE_OUTPUT"]
        else { return }
        let requestData = try Data(contentsOf: URL(fileURLWithPath: requestPath))
        guard
            let request = try JSONSerialization.jsonObject(with: requestData) as? [String: Any]
        else { throw ProbeFailure.message("probe request must be an object") }
        let output: [String: Any]
        if request["operation"] as? String == "describe" {
            output = [
                "spec": "rms/machine-probe-description/v0.1",
                "machine": "SwiftExampleMachine",
                "initial_state": named("Ready"),
                "states": [
                    ["name": "Ready", "data_schema": ["type": "object"]]
                ],
                "inputs": [
                    [
                        "kind": "command",
                        "name": "Describe",
                        "data_schema": [
                            "type": "object",
                            "properties": ["name": ["type": "string", "minLength": 1]],
                            "required": ["name"],
                        ],
                        "example": [
                            "kind": "command",
                            "name": "Describe",
                            "data": ["name": "example"],
                        ],
                    ],
                    [
                        "kind": "command",
                        "name": "RejectEmptyName",
                        "data_schema": [
                            "type": "object",
                            "properties": ["name": ["type": "string"]],
                            "required": ["name"],
                        ],
                        "example": [
                            "kind": "command",
                            "name": "RejectEmptyName",
                            "data": ["name": ""],
                        ],
                    ],
                ],
            ]
        } else {
            let steps = request["steps"] as? [[String: Any]] ?? []
            let records = try steps.enumerated().map { index, step in
                guard let input = step["input"] as? [String: Any] else {
                    throw ProbeFailure.message("step \(index) has no input")
                }
                return recordJSON(
                    transitionRecord(try parseInput(input)),
                    input,
                    scenarioStart: index == 0
                )
            }
            output = [
                "spec": "rms/trace-bundle/v0.1",
                "machine": "SwiftExampleMachine",
                "records": records,
            ]
        }
        let encoded = try JSONSerialization.data(
            withJSONObject: output,
            options: [.prettyPrinted, .sortedKeys]
        )
        try encoded.write(to: URL(fileURLWithPath: outputPath))
    }
}

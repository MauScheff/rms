import Foundation
import XCTest
@testable import RmsSwiftExample

final class RmsSwiftExampleTests: XCTestCase {
    func testRejectsEmptyName() {
        XCTAssertNil(SwiftWidget(""))
    }

    func testAcceptsNonEmptyName() {
        let widget = SwiftWidget("example")

        XCTAssertEqual(widget?.name, "example")
    }

    func testMachineDescribesWidgetName() {
        let widget = SwiftWidget("example")!

        XCTAssertEqual(
            SwiftExampleMachine.transition(.describe(widget)),
            SwiftExampleTransition(
                nextState: .ready,
                events: [.widgetDescribed],
                commands: [],
                effects: [],
                reply: .description("example"),
                rejection: nil
            )
        )
    }

    private func traceCaseName(_ value: Any) -> String {
        let description = String(describing: value)
        let name = description.split(separator: "(", maxSplits: 1).first.map(String.init) ?? description
        return name.prefix(1).uppercased() + name.dropFirst()
    }

    func testProduceTransitionTrace() throws {
        guard let output = ProcessInfo.processInfo.environment["RMS_TRACE_OUTPUT"] else {
            return
        }
        let records = [
            transitionRecord(.describe(SwiftWidget("example")!)),
            transitionRecord(.rejectEmptyName("")),
        ]
        let values: [[String: Any]] = records.map { record in
            [
                "scenario_start": true,
                "state_before": traceCaseName(record.stateBefore),
                "state_after": traceCaseName(record.stateAfter),
                "input": traceCaseName(record.input),
                "output": [
                    "next_state": traceCaseName(record.output.nextState),
                    "events": record.output.events.map { traceCaseName($0) },
                    "commands": record.output.commands.map { traceCaseName($0) },
                    "effects": [],
                    "reply": record.output.reply.map { traceCaseName($0) } ?? NSNull(),
                    "rejection": record.output.rejection.map { traceCaseName($0) } ?? NSNull(),
                ],
                "source": [
                    "file": record.source.file,
                    "function": record.source.function,
                    "branch": record.source.branch,
                ],
            ]
        }
        let document: [String: Any] = [
            "spec": "rms/trace-bundle/v0.1",
            "machine": "SwiftExampleMachine",
            "records": values,
        ]
        let data = try JSONSerialization.data(
            withJSONObject: document,
            options: [.prettyPrinted, .sortedKeys]
        )

        try data.write(to: URL(fileURLWithPath: output))
        XCTAssertEqual(records.count, 2)
    }
}

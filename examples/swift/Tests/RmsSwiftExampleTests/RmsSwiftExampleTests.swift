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

    func testProduceTransitionTrace() throws {
        guard let output = ProcessInfo.processInfo.environment["RMS_TRACE_OUTPUT"] else {
            return
        }
        let widget = SwiftWidget("example")!
        let values: [[String: Any]] = [[
            "spec": "rms/invocation-record/v0.1",
            "contract": "describe-swift-widget",
            "binding": "describe-swift-widget-public",
            "contract_digest": "sha256:f0eb093a9b3bd7e1d763f0c47bb5ca6a15698de7519394850123415fd074b328",
            "scenario_start": true,
            "input": ["kind": "Describe", "widget": ["name": widget.name]],
            "output": ["kind": "Description", "value": describeWidget(widget)],
            "source": [
                "file": #fileID,
                "function": "testProduceTransitionTrace",
                "branch": "accepted-query",
            ],
        ]]
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
        XCTAssertEqual(values.count, 1)
    }
}

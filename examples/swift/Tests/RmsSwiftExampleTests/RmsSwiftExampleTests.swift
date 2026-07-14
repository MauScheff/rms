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
}

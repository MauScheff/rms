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
}

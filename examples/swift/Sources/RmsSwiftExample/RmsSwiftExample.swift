import Foundation

public struct SwiftWidget: Equatable {
    private let rawName: String

    public init?(_ name: String) {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }
        self.rawName = trimmed
    }

    public var name: String { rawName }
}

public func describeWidget(_ widget: SwiftWidget) -> String {
    widget.name
}

public enum SwiftExampleState: Equatable {
    case ready
}

public enum DescribeSwiftWidgetCommand: Equatable {
    case describe(SwiftWidget)
    case rejectEmptyName(String)
}

public enum DescribeSwiftWidgetReply: Equatable {
    case description(String)
    case rejected(reason: String)
}

public enum SwiftExampleMachine {
    public static func transition(_ command: DescribeSwiftWidgetCommand) -> DescribeSwiftWidgetReply {
        switch command {
        case .describe(let widget):
            return .description(describeWidget(widget))
        case .rejectEmptyName(let name):
            if SwiftWidget(name) == nil {
                return .rejected(reason: "empty-swift-widget-name")
            }
            return .rejected(reason: "expected-empty-name")
        }
    }
}

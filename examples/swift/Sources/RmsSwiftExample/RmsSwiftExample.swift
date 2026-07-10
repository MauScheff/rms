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

public enum SwiftExampleEvent: Equatable {
    case widgetDescribed
    case widgetRejected
}

public enum SwiftExampleRejection: Equatable {
    case emptyWidgetName
    case expectedEmptyName
}

public enum DescribeSwiftWidgetReply: Equatable {
    case description(String)
    case rejected(reason: SwiftExampleRejection)
}

public struct SwiftExampleTransition: Equatable {
    public let nextState: SwiftExampleState
    public let events: [SwiftExampleEvent]
    public let commands: [DescribeSwiftWidgetCommand]
    public let effects: [Never]
    public let reply: DescribeSwiftWidgetReply
}

public struct SwiftExampleTransitionRecord: Equatable {
    public let stateBefore: SwiftExampleState
    public let stateAfter: SwiftExampleState
    public let input: DescribeSwiftWidgetCommand
    public let output: SwiftExampleTransition
}

public enum SwiftExampleMachine {
    public static func transition(_ command: DescribeSwiftWidgetCommand) -> SwiftExampleTransition {
        RmsSwiftExample.transition(command)
    }
}

public func transition(_ command: DescribeSwiftWidgetCommand) -> SwiftExampleTransition {
    transitionRecord(command).output
}

public func transitionRecord(_ command: DescribeSwiftWidgetCommand) -> SwiftExampleTransitionRecord {
    let stateBefore = SwiftExampleState.ready
    let event: SwiftExampleEvent
    let reply: DescribeSwiftWidgetReply
    switch command {
    case .describe(let widget):
        event = .widgetDescribed
        reply = .description(describeWidget(widget))
    case .rejectEmptyName(let name):
        event = .widgetRejected
        reply = .rejected(
            reason: SwiftWidget(name) == nil ? .emptyWidgetName : .expectedEmptyName
        )
    }
    let output = SwiftExampleTransition(
        nextState: .ready,
        events: [event],
        commands: [],
        effects: [],
        reply: reply
    )
    return SwiftExampleTransitionRecord(
        stateBefore: stateBefore,
        stateAfter: output.nextState,
        input: command,
        output: output
    )
}

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

// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "rms-swift-example",
    products: [
        .library(name: "RmsSwiftExample", targets: ["RmsSwiftExample"])
    ],
    targets: [
        .target(name: "RmsSwiftExample"),
        .testTarget(name: "RmsSwiftExampleTests", dependencies: ["RmsSwiftExample"])
    ]
)

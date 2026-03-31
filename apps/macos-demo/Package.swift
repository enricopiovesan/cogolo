// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "TraverseMacOSDemo",
    platforms: [
        .macOS(.v14),
    ],
    products: [
        .executable(name: "TraverseMacOSDemoApp", targets: ["TraverseMacOSDemoApp"]),
    ],
    targets: [
        .executableTarget(
            name: "TraverseMacOSDemoApp",
            path: "Sources/TraverseMacOSDemoApp"
        ),
    ]
)

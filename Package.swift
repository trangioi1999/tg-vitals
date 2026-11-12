// swift-tools-version:5.10
import PackageDescription

let package = Package(
    name: "TGVitals",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "TGVitals", targets: ["TGVitals"]),
    ],
    targets: [
        // Data collection + cleanup logic. No SwiftUI here, fully testable.
        .target(
            name: "VitalsCore",
            path: "Sources/VitalsCore"
        ),
        // The app: menu bar, windows, views.
        .executableTarget(
            name: "TGVitals",
            dependencies: ["VitalsCore"],
            path: "Sources/TGVitals"
        ),
        .testTarget(
            name: "VitalsCoreTests",
            dependencies: ["VitalsCore"],
            path: "Tests/VitalsCoreTests"
        ),
    ]
)

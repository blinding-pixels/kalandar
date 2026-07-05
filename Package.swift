// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "Kalandar",
    platforms: [
        .macOS(.v13),
        .iOS(.v16),
    ],
    products: [
        .library(
            name: "KalandarCoreAIRuntime",
            targets: ["KalandarCoreAIRuntime"]
        ),
        .executable(
            name: "kalandar-coreai-capture-preview",
            targets: ["kalandar-coreai-capture-preview"]
        ),
    ],
    targets: [
        .target(
            name: "KalandarCoreAIRuntime",
            path: "swift/Sources/KalandarCoreAIRuntime",
            swiftSettings: [
                .enableUpcomingFeature("MemberImportVisibility"),
            ]
        ),
        .executableTarget(
            name: "kalandar-coreai-capture-preview",
            dependencies: ["KalandarCoreAIRuntime"],
            path: "swift/Sources/kalandar-coreai-capture-preview",
            swiftSettings: [
                .enableUpcomingFeature("MemberImportVisibility"),
            ]
        ),
        .testTarget(
            name: "KalandarCoreAIRuntimeTests",
            dependencies: ["KalandarCoreAIRuntime"],
            path: "swift/Tests/KalandarCoreAIRuntimeTests"
        ),
    ]
)

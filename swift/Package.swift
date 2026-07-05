// swift-tools-version: 6.0

import PackageDescription

let package = Package(
    name: "KalandarCoreAIRuntime",
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
            swiftSettings: [
                .enableUpcomingFeature("MemberImportVisibility"),
            ]
        ),
        .executableTarget(
            name: "kalandar-coreai-capture-preview",
            dependencies: ["KalandarCoreAIRuntime"],
            swiftSettings: [
                .enableUpcomingFeature("MemberImportVisibility"),
            ]
        ),
        .testTarget(
            name: "KalandarCoreAIRuntimeTests",
            dependencies: ["KalandarCoreAIRuntime"]
        ),
    ]
)

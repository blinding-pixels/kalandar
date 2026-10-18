// swift-tools-version: 6.0

import Foundation
import PackageDescription

let kalandarDatesVersion = "0.2.1"
let kalandarDatesChecksum = "c6da8f9d84285c360ee9bc067f019dbbe6f90ff406b4448412eaeb5ef58af168"
let localKalandarDatesFFI = "swift/Binaries/KalandarDatesFFI.xcframework"
let kalandarDatesFFI: Target = FileManager.default.fileExists(atPath: "\(Context.packageDirectory)/\(localKalandarDatesFFI)")
    ? .binaryTarget(name: "KalandarDatesFFI", path: localKalandarDatesFFI)
    : .binaryTarget(
        name: "KalandarDatesFFI",
        url: "https://github.com/blinding-pixels/kalandar/releases/download/v\(kalandarDatesVersion)/KalandarDatesFFI-\(kalandarDatesVersion).xcframework.zip",
        checksum: kalandarDatesChecksum
    )

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
        .library(
            name: "KalandarDates",
            targets: ["KalandarDates"]
        ),
        .executable(
            name: "kalandar-coreai-capture-preview",
            targets: ["kalandar-coreai-capture-preview"]
        ),
        .executable(
            name: "kalandar-dates-schedule",
            targets: ["kalandar-dates-schedule"]
        ),
    ],
    targets: [
        kalandarDatesFFI,
        .target(
            name: "KalandarDates",
            dependencies: ["KalandarDatesFFI"],
            path: "swift/Sources/KalandarDates",
            swiftSettings: [
                .swiftLanguageMode(.v5),
            ]
        ),
        .target(
            name: "KalandarCoreAIRuntime",
            dependencies: ["KalandarDates"],
            path: "swift/Sources/KalandarCoreAIRuntime",
            swiftSettings: [
                .enableUpcomingFeature("MemberImportVisibility"),
            ]
        ),
        .executableTarget(
            name: "kalandar-coreai-capture-preview",
            dependencies: ["KalandarCoreAIRuntime", "KalandarDates"],
            path: "swift/Sources/kalandar-coreai-capture-preview",
            swiftSettings: [
                .enableUpcomingFeature("MemberImportVisibility"),
            ]
        ),
        .executableTarget(
            name: "kalandar-dates-schedule",
            dependencies: ["KalandarDates"],
            path: "swift/Sources/kalandar-dates-schedule",
            swiftSettings: [
                .swiftLanguageMode(.v5),
            ]
        ),
        .testTarget(
            name: "KalandarCoreAIRuntimeTests",
            dependencies: ["KalandarCoreAIRuntime", "KalandarDates"],
            path: "swift/Tests/KalandarCoreAIRuntimeTests",
            resources: [
                .copy("Fixtures"),
            ]
        ),
    ]
)

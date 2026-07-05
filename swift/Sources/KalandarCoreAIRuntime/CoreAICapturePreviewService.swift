import Foundation

public struct CoreAICapturePreviewService: Sendable {
    private let parser: LightweightCaptureParser
    private let store: CoreAIArtifactStore?
    private let smokeVerifier: CoreAISmokeVerifier
    private let requireCoreAISmoke: Bool

    public init(
        artifactRoot: URL? = nil,
        requireCoreAISmoke: Bool = false,
        smokeVerifier: CoreAISmokeVerifier = CoreAISmokeVerifier()
    ) throws {
        if let artifactRoot {
            self.store = try CoreAIArtifactStore(root: artifactRoot)
        } else {
            self.store = nil
        }
        self.parser = LightweightCaptureParser()
        self.smokeVerifier = smokeVerifier
        self.requireCoreAISmoke = requireCoreAISmoke
    }

    public func preview(_ request: CapturePreviewRequest) async throws -> CapturePreviewResult {
        if requireCoreAISmoke {
            try await verifyAvailableModel(for: request.text)
        }
        return parser.parse(request)
    }

    private func verifyAvailableModel(for text: String) async throws {
        guard let store else {
            throw KalandarCoreAIError.artifactRootMissing("missing KALANDAR_COREAI_ARTIFACT_ROOT")
        }
        let family = familyForSmoke(text)
        let artifact = try store.artifact(family: family)
        try await smokeVerifier.verify(artifact: artifact, sample: smokeSample(family))
    }

    private func familyForSmoke(_ text: String) -> String {
        let lowered = TextUtilities.shadow(text)
        if lowered.hasPrefix("reminder:") || lowered.hasPrefix("remind me") {
            return "reminder"
        }
        if lowered.hasPrefix("event:") || lowered.contains("meeting") || lowered.contains("lunch") {
            return "calendar"
        }
        return "task"
    }

    private func smokeSample(_ family: String) -> String {
        switch family {
        case "calendar":
            return "lunch with sara tomorrow at noon"
        case "reminder":
            return "remind me to water plants tomorrow at 9"
        default:
            return "finish report tomorrow"
        }
    }
}

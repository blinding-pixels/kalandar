import Foundation

public struct CoreAICapturePreviewService: Sendable {
    private let parser: LightweightCaptureParser
    private let suite: CoreAIParserSuite?

    public init(
        artifactRoot: URL? = nil,
        requireCoreAISmoke: Bool = false,
        tensorRunner: any CoreAITensorRunning = CoreAITensorRunner(),
        smokeVerifier: CoreAISmokeVerifier? = nil
    ) throws {
        if let artifactRoot {
            self.suite = try CoreAIParserSuite(
                artifactRoot: artifactRoot,
                requireCoreAISmoke: requireCoreAISmoke,
                tensorRunner: tensorRunner,
                smokeVerifier: smokeVerifier
            )
        } else {
            self.suite = nil
        }
        self.parser = LightweightCaptureParser()
    }

    public func preview(_ request: CapturePreviewRequest) async throws -> CapturePreviewResult {
        let fallback = parser.parse(request)
        guard let suite else {
            return fallback
        }
        switch fallback.kind {
        case "task", "reminder", "calendar-event":
            var result = try await suite.parsePreviewResult(for: fallback.kind, request: request)
            result.confidence = fallback.confidence
            result.candidates = fallback.candidates
            return result
        default:
            return fallback
        }
    }
}

import Foundation

public enum CoreAIParserFamily: String, CaseIterable, Codable, Sendable {
    case task
    case reminder
    case calendar

    fileprivate var capturePrefix: String {
        switch self {
        case .task: "task:"
        case .reminder: "reminder:"
        case .calendar: "event:"
        }
    }
}

public struct CoreAIModelInfo: Equatable, Sendable {
    public let family: CoreAIParserFamily
    public let manifestVersion: String
    public let modelKind: String
    public let labelCount: Int
    public let weightQuantization: String?
}

public struct CoreAIParsed<Value: Sendable>: Sendable {
    public let value: Value
    public let entities: [CoreAIEntity]
    public let model: CoreAIModelInfo
}

/// Typed entry points for the task, reminder, and calendar Core AI models.
public struct CoreAIParserSuite: Sendable {
    private let store: CoreAIArtifactStore
    private let lightweightParser: LightweightCaptureParser
    private let tensorRunner: any CoreAITensorRunning
    private let smokeVerifier: CoreAISmokeVerifier
    private let requireCoreAISmoke: Bool

    public init(
        artifactRoot: URL,
        requireCoreAISmoke: Bool = false,
        tensorRunner: any CoreAITensorRunning = CoreAITensorRunner(),
        smokeVerifier: CoreAISmokeVerifier? = nil
    ) throws {
        self.store = try CoreAIArtifactStore(root: artifactRoot)
        self.lightweightParser = LightweightCaptureParser()
        self.tensorRunner = tensorRunner
        self.smokeVerifier = smokeVerifier ?? CoreAISmokeVerifier(runner: tensorRunner)
        self.requireCoreAISmoke = requireCoreAISmoke
    }

    public func modelInfo(for family: CoreAIParserFamily) throws -> CoreAIModelInfo {
        try info(for: store.artifact(family: family.rawValue), family: family)
    }

    public func parseTask(_ request: CapturePreviewRequest) async throws -> CoreAIParsed<CapturePreviewTaskDraft> {
        let parsed = try await parsePreview(.task, request: request)
        guard let value = parsed.value.task else {
            throw KalandarCoreAIError.invalidModelOutput("task model did not produce a task draft")
        }
        return CoreAIParsed(value: value, entities: parsed.entities, model: parsed.model)
    }

    public func parseReminder(_ request: CapturePreviewRequest) async throws -> CoreAIParsed<CapturePreviewReminderDraft> {
        let parsed = try await parsePreview(.reminder, request: request)
        guard let value = parsed.value.reminder else {
            throw KalandarCoreAIError.invalidModelOutput("reminder model did not produce a reminder draft")
        }
        return CoreAIParsed(value: value, entities: parsed.entities, model: parsed.model)
    }

    public func parseCalendar(_ request: CapturePreviewRequest) async throws -> CoreAIParsed<CapturePreviewCalendarEventDraft> {
        let parsed = try await parsePreview(.calendar, request: request)
        guard let value = parsed.value.event else {
            throw KalandarCoreAIError.invalidModelOutput("calendar model did not produce an event draft")
        }
        return CoreAIParsed(value: value, entities: parsed.entities, model: parsed.model)
    }

    private func parsePreview(
        _ family: CoreAIParserFamily,
        request: CapturePreviewRequest
    ) async throws -> CoreAIParsed<CapturePreviewResult> {
        let artifact = try store.artifact(family: family.rawValue)
        let model = try info(for: artifact, family: family)
        if requireCoreAISmoke {
            try await smokeVerifier.verify(artifact: artifact, sample: smokeSample(for: family))
        }

        let input = modelInput(from: request.text, family: family)
        var fallbackRequest = request
        fallbackRequest.text = "\(family.capturePrefix) \(input)"
        let fallback = lightweightParser.parse(fallbackRequest)

        let tokenized = try BertWordPieceTokenizer(artifact: artifact).encode(input)
        let output = try await tensorRunner.runTokenClassifier(artifact: artifact, tokenized: tokenized)
        let entities = try CoreAIEntityDecoder.decode(
            output: output,
            tokenized: tokenized,
            labels: artifact.manifest.labels ?? [:],
            sourceText: input
        )
        return CoreAIParsed(
            value: CoreAIEntityMerger.apply(entities, to: fallback),
            entities: entities,
            model: model
        )
    }

    private func info(for artifact: CoreAIParserArtifact, family: CoreAIParserFamily) throws -> CoreAIModelInfo {
        guard artifact.manifest.modelKind == "token-classification" else {
            throw KalandarCoreAIError.invalidManifest("\(family.rawValue) artifact is not a token-classification model")
        }
        guard let labels = artifact.manifest.labels, !labels.isEmpty else {
            throw KalandarCoreAIError.invalidManifest("\(family.rawValue) artifact is missing token-classification labels")
        }
        return CoreAIModelInfo(
            family: family,
            manifestVersion: artifact.manifest.manifestVersion,
            modelKind: artifact.manifest.modelKind,
            labelCount: labels.count,
            weightQuantization: artifact.manifest.optimization?.weightQuantization
        )
    }

    private func modelInput(from text: String, family: CoreAIParserFamily) -> String {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.lowercased().hasPrefix(family.capturePrefix) else { return trimmed }
        return String(trimmed.dropFirst(family.capturePrefix.count)).trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func smokeSample(for family: CoreAIParserFamily) -> String {
        switch family {
        case .task: "finish report tomorrow"
        case .reminder: "remind me to water plants tomorrow at 9"
        case .calendar: "lunch with sara tomorrow at noon"
        }
    }
}

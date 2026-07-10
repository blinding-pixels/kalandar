import Foundation
import Testing
@testable import KalandarCoreAIRuntime

@Suite("Core AI token-classification parsing")
struct CoreAIModelParsingTests {
    @Test("decodes BIO logits into source entities")
    func decodesBioLogits() throws {
        let tokenized = TokenizedText(
            inputIds: [101, 10, 11, 12, 102, 0],
            attentionMask: [1, 1, 1, 1, 1, 0],
            offsets: [(0, 0), (0, 6), (7, 13), (14, 22), (0, 0), (0, 0)],
            truncated: false
        )
        let output = CoreAITensorOutput(
            name: "logits",
            shape: [1, 6, 4],
            values: logits(tokenCount: 6, labelCount: 4, labels: [0, 1, 2, 3, 0, 0])
        )

        let entities = try CoreAIEntityDecoder.decode(
            output: output,
            tokenized: tokenized,
            labels: ["0": "O", "1": "B-title", "2": "I-title", "3": "B-due_date"],
            sourceText: "finish report tomorrow"
        )

        #expect(entities.count == 2)
        #expect(entities[0].field == "title")
        #expect(entities[0].text == "finish report")
        #expect(entities[0].startOffset == 0)
        #expect(entities[0].endOffset == 13)
        #expect(entities[1].field == "due_date")
        #expect(entities[1].text == "tomorrow")
        #expect(entities[1].startOffset == 14)
        #expect(entities[1].endOffset == 22)
        #expect(entities.allSatisfy { $0.confidence > 0.99 })
    }

    @Test("uses a token-classification artifact for capture preview")
    func serviceAppliesModelEntities() async throws {
        let root = try makeArtifactRoot()
        defer { try? FileManager.default.removeItem(at: root) }

        let service = try CoreAICapturePreviewService(
            artifactRoot: root,
            tensorRunner: FixtureTensorRunner()
        )
        let result = try await service.preview(CapturePreviewRequest(text: "task: finish report tomorrow"))

        #expect(result.kind == "task")
        #expect(result.task?.title == "finish report")
        #expect(result.task?.dueDateText == "tomorrow")
    }

    @Test("exposes a typed task model API")
    func suiteParsesTaskWithTaskArtifact() async throws {
        let root = try makeArtifactRoot()
        defer { try? FileManager.default.removeItem(at: root) }

        let suite = try CoreAIParserSuite(artifactRoot: root, tensorRunner: FixtureTensorRunner())
        let parsed = try await suite.parseTask(CapturePreviewRequest(text: "finish report tomorrow"))

        #expect(parsed.model.family == .task)
        #expect(parsed.model.modelKind == "token-classification")
        #expect(parsed.model.labelCount == 4)
        #expect(parsed.value.title == "finish report")
        #expect(parsed.value.dueDateText == "tomorrow")
        #expect(parsed.entities.map(\.field) == ["title", "due_date"])
    }

    @Test("maps every trained reminder and calendar field")
    func mapsReminderLeadTimeAndCalendarDescription() {
        let reminder = CoreAIEntityMerger.apply(
            [CoreAIEntity(field: "lead_time", text: "15 minutes", startOffset: 0, endOffset: 10, confidence: 0.9)],
            to: CapturePreviewResult(
                kind: "reminder",
                confidence: 1,
                reminder: CapturePreviewReminderDraft(title: "water plants")
            )
        )
        #expect(reminder.reminder?.leadTimeText == "15 minutes")

        let calendar = CoreAIEntityMerger.apply(
            [CoreAIEntity(field: "description", text: "bring the draft", startOffset: 0, endOffset: 15, confidence: 0.9)],
            to: CapturePreviewResult(
                kind: "calendar-event",
                confidence: 1,
                event: CapturePreviewCalendarEventDraft(title: "review")
            )
        )
        #expect(calendar.event?.descriptionText == "bring the draft")
    }

    private func makeArtifactRoot() throws -> URL {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("kalandar-coreai-fixture-\(UUID().uuidString)", isDirectory: true)
        let taskRoot = root.appendingPathComponent("task", isDirectory: true)
        let tokenizerRoot = taskRoot.appendingPathComponent("tokenizer", isDirectory: true)
        try FileManager.default.createDirectory(at: tokenizerRoot, withIntermediateDirectories: true)
        try Data().write(to: taskRoot.appendingPathComponent("model.aimodel"))
        try "[PAD]\n[UNK]\n[CLS]\n[SEP]\ntask\n:\nfinish\nreport\ntomorrow\n".write(
            to: tokenizerRoot.appendingPathComponent("vocab.txt"),
            atomically: true,
            encoding: .utf8
        )

        let manifest = """
        {
          "manifest_version": "kalandar-coreai-model-v1",
          "runtime_kind": "coreai",
          "model_kind": "token-classification",
          "model_asset": "model.aimodel",
          "input_contract": {
            "inputs": [
              { "name": "input_ids", "dtype": "int32", "shape": [1, 8] },
              { "name": "attention_mask", "dtype": "int32", "shape": [1, 8] }
            ],
            "outputs": [
              { "name": "logits", "dtype": "float32", "shape": [1, 8, 4] }
            ],
            "max_sequence_length": 8
          },
          "labels": {
            "0": "O",
            "1": "B-title",
            "2": "I-title",
            "3": "B-due_date"
          },
          "tokenizer": { "kind": "bert_wordpiece", "path": "tokenizer" }
        }
        """
        try manifest.write(to: taskRoot.appendingPathComponent("manifest.json"), atomically: true, encoding: .utf8)
        return root
    }
}

private struct FixtureTensorRunner: CoreAITensorRunning {
    func runTokenClassifier(artifact _: CoreAIParserArtifact, tokenized: TokenizedText) async throws -> CoreAITensorOutput {
        let tokenLabels = tokenized.offsets.map { offset -> Int in
            switch offset {
            case (0, 6): return 1
            case (7, 13): return 2
            case (14, 22): return 3
            default: return 0
            }
        }
        return CoreAITensorOutput(
            name: "logits",
            shape: [1, tokenized.inputIds.count, 4],
            values: logits(tokenCount: tokenized.inputIds.count, labelCount: 4, labels: tokenLabels)
        )
    }
}

private func logits(tokenCount: Int, labelCount: Int, labels: [Int]) -> [Float] {
    var values = Array(repeating: Float(-5), count: tokenCount * labelCount)
    for (tokenIndex, labelID) in labels.enumerated() {
        values[tokenIndex * labelCount + labelID] = 5
    }
    return values
}

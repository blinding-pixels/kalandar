import Foundation
import Testing
@testable import KalandarCoreAIRuntime

private struct GoldenCase: Decodable {
    var labels: [String]
    var logits: [[Double]]
    var argmax: [Int]
    var bio: [Int]
}

private let smallLabels = ["O", "B-a", "I-a", "B-b", "I-b"]

private func fixtureData(_ name: String) throws -> Data {
    guard let url = Bundle.module.url(forResource: name, withExtension: "json", subdirectory: "Fixtures") else {
        throw KalandarCoreAIError.artifactMissing("Fixtures/\(name).json")
    }
    return try Data(contentsOf: url)
}

private func goldenCases() throws -> [GoldenCase] {
    try JSONDecoder().decode([GoldenCase].self, from: fixtureData("decoder_golden"))
}

private func isLegalPath(_ labels: [String], _ path: [Int]) -> Bool {
    path.indices.allSatisfy { position in
        let label = labels[path[position]]
        guard label.hasPrefix("I-") else { return true }
        guard position > 0 else { return false }
        let previous = labels[path[position - 1]]
        return previous == label || previous == "B-" + label.dropFirst(2)
    }
}

private func allPaths(length: Int, labelCount: Int) -> [[Int]] {
    (0..<length).reduce([[Int]]([[]])) { paths, _ in
        paths.flatMap { prefix in (0..<labelCount).map { prefix + [$0] } }
    }
}

private func pathScore(_ logits: [[Double]], _ path: [Int]) -> Double {
    path.enumerated().reduce(0) { $0 + logits[$1.offset][$1.element] }
}

@Suite("First-piece sequence decoding")
struct SequenceDecoderTests {
    @Test("golden cases match the reference argmax and BIO paths")
    func goldenCasesMatch() throws {
        let cases = try goldenCases()
        #expect(cases.count == 6)
        for golden in cases {
            #expect(try CoreAISequenceDecoder.decodeArgmax(golden.logits, labels: golden.labels) == golden.argmax)
            #expect(try CoreAISequenceDecoder.decodeBioViterbi(golden.logits, labels: golden.labels) == golden.bio)
        }
    }

    @Test("BIO Viterbi finds a best legal path on exhaustive cases")
    func exhaustiveCases() throws {
        var state: UInt32 = 729
        func nextUnit() -> Double {
            state = state &* 1_664_525 &+ 1_013_904_223
            return Double(state) / 4_294_967_296
        }
        for length in 1...4 {
            let candidates = allPaths(length: length, labelCount: smallLabels.count).filter { isLegalPath(smallLabels, $0) }
            for sample in 0..<25 {
                let logits = (0..<length).map { _ in
                    (0..<smallLabels.count).map { _ in sample % 5 == 0 ? (nextUnit() * 2).rounded() : nextUnit() * 4 - 2 }
                }
                let actual = try CoreAISequenceDecoder.decodeBioViterbi(logits, labels: smallLabels)
                let best = candidates.map { pathScore(logits, $0) }.max() ?? -.infinity
                #expect(isLegalPath(smallLabels, actual))
                #expect(pathScore(logits, actual) == best)
            }
        }
    }

    @Test("handles empty sequences, ties and illegal starts")
    func edgeCases() throws {
        #expect(try CoreAISequenceDecoder.decodeBioViterbi([], labels: smallLabels) == [])
        #expect(try CoreAISequenceDecoder.decodeBioViterbi(Array(repeating: [0, 0, 0, 0, 0], count: 3), labels: smallLabels) == [0, 0, 0])
        #expect(try CoreAISequenceDecoder.decodeBioViterbi([[0, 0, 5, 0, 0]], labels: smallLabels) == [0])
        #expect(try CoreAISequenceDecoder.decodeBioViterbi([[0, 3, 0, 0, 0], [0, 0, 0, 0, 5]], labels: smallLabels) == [3, 4])
    }

    @Test("rejects invalid labels and logits")
    func rejectsInvalidInput() {
        #expect(throws: KalandarCoreAIError.self) { try CoreAISequenceDecoder.decodeBioViterbi([[0, 0]], labels: ["O"]) }
        #expect(throws: KalandarCoreAIError.self) { try CoreAISequenceDecoder.decodeBioViterbi([[.nan]], labels: ["O"]) }
        #expect(throws: KalandarCoreAIError.self) { try CoreAISequenceDecoder.decodeArgmax([[.infinity, 0]], labels: ["O", "B-a"]) }
        for labels in [["O", "I-a"], ["O", "X-a"], ["O", "O"], ["B-a", "I-a"]] {
            #expect(throws: KalandarCoreAIError.self) { try CoreAISequenceDecoder.decodeBioViterbi([[1, 2]], labels: labels) }
        }
    }

    @Test("splits whitespace words and aligns first pieces")
    func alignsFirstPieces() {
        let words = CoreAISequenceDecoder.whitespaceWords(Array("  lunch\u{a0}with\tsara\u{1f}x\n"))
        #expect(words == [
            .init(start: 2, end: 7),
            .init(start: 8, end: 12),
            .init(start: 13, end: 17),
            .init(start: 18, end: 19),
        ])
        let groceries = CoreAISequenceDecoder.whitespaceWords(Array("pick up groceries, today"))
        let offsets = [(0, 0), (0, 4), (5, 7), (8, 13), (13, 17), (17, 18), (19, 24), (0, 0)]
        #expect(CoreAISequenceDecoder.alignFirstPieces(words: groceries, offsets: offsets) == [1, 2, 3, 6])
        let truncated = CoreAISequenceDecoder.whitespaceWords(Array("one two three"))
        #expect(CoreAISequenceDecoder.alignFirstPieces(words: truncated, offsets: [(0, 0), (0, 3), (0, 0)]) == [1, nil, nil])
    }

    @Test("rebuilds spans from whitespace words")
    func rebuildsSpans() {
        let words = CoreAISequenceDecoder.whitespaceWords(Array("a b c d e f"))
        let spans = CoreAISequenceDecoder.rebuildSpans(words: words, wordLabels: ["B-x", "B-x", "I-x", "O", "I-x", "I-y"])
        #expect(spans.map(\.field) == ["x", "x", "x", "y"])
        #expect(spans.map { [$0.start, $0.end] } == [[0, 1], [2, 5], [8, 9], [10, 11]])
    }

    @Test("decodes token logits through first pieces with the declared strategy")
    func decodesFirstPieces() throws {
        let labels = ["0": "O", "1": "B-title", "2": "I-title", "3": "B-due_date", "4": "I-due_date"]
        let tokenized = TokenizedText(
            inputIds: [101, 10, 11, 12, 13, 102, 0],
            attentionMask: [1, 1, 1, 1, 1, 1, 0],
            offsets: [(0, 0), (0, 6), (7, 13), (13, 14), (15, 23), (0, 0), (0, 0)],
            truncated: false
        )
        let tokenLabels = [0, 1, 2, 3, 4, 0, 0]
        var values = [Float](repeating: 0, count: 7 * 5)
        for (token, label) in tokenLabels.enumerated() {
            values[token * 5 + label] = 9
        }
        let output = CoreAITensorOutput(name: "logits", shape: [1, 7, 5], values: values)

        let argmax = try CoreAIEntityDecoder.decode(
            output: output, tokenized: tokenized, labels: labels, sourceText: "finish reports tomorrow", decoding: .argmax
        )
        #expect(argmax.map(\.field) == ["title", "due_date"])
        #expect(argmax.map(\.text) == ["finish reports", "tomorrow"])

        let bio = try CoreAIEntityDecoder.decode(
            output: output, tokenized: tokenized, labels: labels, sourceText: "finish reports tomorrow", decoding: .bio
        )
        #expect(bio.map(\.field) == ["title"])
        #expect(bio.map(\.text) == ["finish reports"])
        #expect(bio[0].startOffset == 0 && bio[0].endOffset == 14)
    }

    @Test("accepts the shipped calendar sidecar and rejects drift")
    func validatesSidecar() throws {
        let data = try fixtureData("calendar_sequence_decoder")
        let payload = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        let labels = payload?["labels"] as? [String] ?? []
        #expect(labels.count == 21)
        try CoreAISequenceDecoder.validateSidecar(data, labels: labels)
        #expect(throws: KalandarCoreAIError.self) {
            try CoreAISequenceDecoder.validateSidecar(data, labels: Array(labels.dropLast(2)))
        }
    }

    @Test("loads decoding and the sidecar from an artifact manifest")
    func loadsArtifactDecoding() throws {
        let root = try makeArtifactRoot(decoding: "bio", sidecar: fixtureData("calendar_sequence_decoder"))
        defer { try? FileManager.default.removeItem(at: root) }
        let artifact = try CoreAIParserArtifact.load(family: "calendar", from: root)
        #expect(artifact.decoding == .bio)
        #expect(artifact.sequenceDecoderURL?.lastPathComponent == "sequence_decoder.json")

        let plain = try makeArtifactRoot(decoding: nil, sidecar: nil)
        defer { try? FileManager.default.removeItem(at: plain) }
        #expect(try CoreAIParserArtifact.load(family: "calendar", from: plain).decoding == .argmax)

        let invalid = try makeArtifactRoot(decoding: "viterbi", sidecar: nil)
        defer { try? FileManager.default.removeItem(at: invalid) }
        #expect(throws: KalandarCoreAIError.self) { try CoreAIParserArtifact.load(family: "calendar", from: invalid) }

        let tampered = try makeArtifactRoot(decoding: "bio", sidecar: Data("{}".utf8))
        defer { try? FileManager.default.removeItem(at: tampered) }
        #expect(throws: (any Error).self) { try CoreAIParserArtifact.load(family: "calendar", from: tampered) }
    }

    private func makeArtifactRoot(decoding: String?, sidecar: Data?) throws -> URL {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("kalandar-coreai-decoding-\(UUID().uuidString)", isDirectory: true)
        let familyRoot = root.appendingPathComponent("calendar", isDirectory: true)
        let tokenizerRoot = familyRoot.appendingPathComponent("tokenizer", isDirectory: true)
        try FileManager.default.createDirectory(at: tokenizerRoot, withIntermediateDirectories: true)
        try Data().write(to: familyRoot.appendingPathComponent("model.aimodel"))
        try "[PAD]\n[UNK]\n[CLS]\n[SEP]\n".write(to: tokenizerRoot.appendingPathComponent("vocab.txt"), atomically: true, encoding: .utf8)
        if let sidecar {
            try sidecar.write(to: familyRoot.appendingPathComponent("sequence_decoder.json"))
        }

        let labels = try JSONSerialization.jsonObject(with: fixtureData("calendar_sequence_decoder")) as? [String: Any]
        let labelMap = Dictionary(uniqueKeysWithValues: (labels?["labels"] as? [String] ?? []).enumerated().map { (String($0.offset), $0.element) })
        var manifest: [String: Any] = [
            "manifest_version": "kalandar-coreai-model-v1",
            "runtime_kind": "coreai",
            "model_kind": "token-classification",
            "model_asset": "model.aimodel",
            "input_contract": [
                "inputs": [
                    ["name": "input_ids", "dtype": "int32", "shape": [1, 128]],
                    ["name": "attention_mask", "dtype": "int32", "shape": [1, 128]],
                ],
                "outputs": [["name": "logits", "dtype": "float32", "shape": [1, 128, 21]]],
                "max_sequence_length": 128,
            ],
            "labels": labelMap,
            "tokenizer": ["kind": "bert_wordpiece", "path": "tokenizer"],
        ]
        if let decoding {
            manifest["decoding"] = decoding
        }
        if sidecar != nil {
            manifest["sequence_decoder"] = "sequence_decoder.json"
        }
        try JSONSerialization.data(withJSONObject: manifest).write(to: familyRoot.appendingPathComponent("manifest.json"))
        return root
    }
}

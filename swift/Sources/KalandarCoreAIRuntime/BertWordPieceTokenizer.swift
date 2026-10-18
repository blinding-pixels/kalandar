import Foundation

public struct TokenizedText: Sendable {
    public var inputIds: [Int32]
    public var attentionMask: [Int32]
    public var offsets: [(Int, Int)]
    public var truncated: Bool
}

public struct BertWordPieceTokenizer: Sendable {
    private let vocab: [String: Int32]
    private let clsId: Int32
    private let sepId: Int32
    private let padId: Int32
    private let unkId: Int32
    private let maxSequenceLength: Int

    public init(vocabURL: URL, maxSequenceLength: Int) throws {
        let text = try String(contentsOf: vocabURL, encoding: .utf8)
        var vocab: [String: Int32] = [:]
        for (index, line) in text.split(separator: "\n", omittingEmptySubsequences: false).enumerated() {
            let token = String(line.trimmingCharacters(in: .newlines))
            if !token.isEmpty {
                vocab[token] = Int32(index)
            }
        }
        guard let clsId = vocab["[CLS]"], let sepId = vocab["[SEP]"], let padId = vocab["[PAD]"], let unkId = vocab["[UNK]"] else {
            throw KalandarCoreAIError.invalidManifest("tokenizer vocab is missing required special tokens")
        }
        self.vocab = vocab
        self.clsId = clsId
        self.sepId = sepId
        self.padId = padId
        self.unkId = unkId
        self.maxSequenceLength = maxSequenceLength
    }

    public init(artifact: CoreAIParserArtifact) throws {
        try self.init(
            vocabURL: artifact.tokenizerURL.appendingPathComponent("vocab.txt"),
            maxSequenceLength: artifact.manifest.inputContract.maxSequenceLength
        )
    }

    public func encode(_ input: String) -> TokenizedText {
        let tokens = basicTokenize(input)
        let maxPieces = max(0, maxSequenceLength - 2)
        var pieces: [(Int32, Int, Int)] = []
        var truncated = false

        for token in tokens {
            let encoded = encodeWordPiece(token)
            if pieces.count + encoded.count > maxPieces {
                pieces.append(contentsOf: encoded.prefix(maxPieces - pieces.count))
                truncated = true
                break
            }
            pieces.append(contentsOf: encoded)
        }

        var ids = [clsId] + pieces.map(\.0) + [sepId]
        var mask = Array(repeating: Int32(1), count: ids.count)
        var offsets = [(0, 0)] + pieces.map { ($0.1, $0.2) } + [(0, 0)]

        if ids.count < maxSequenceLength {
            let padding = maxSequenceLength - ids.count
            ids.append(contentsOf: Array(repeating: padId, count: padding))
            mask.append(contentsOf: Array(repeating: Int32(0), count: padding))
            offsets.append(contentsOf: Array(repeating: (0, 0), count: padding))
        }

        return TokenizedText(inputIds: ids, attentionMask: mask, offsets: offsets, truncated: truncated)
    }

    private typealias BasicToken = (String, [(Unicode.Scalar, Int, Int)])

    private static let maxInputCharsPerWord = 100

    private static func isControl(_ scalar: Unicode.Scalar) -> Bool {
        if scalar == "\t" || scalar == "\n" || scalar == "\r" {
            return false
        }
        switch scalar.properties.generalCategory {
        case .control, .format, .privateUse, .unassigned, .surrogate:
            return true
        default:
            return false
        }
    }

    private static func isWhitespace(_ scalar: Unicode.Scalar) -> Bool {
        scalar == "\t" || scalar == "\n" || scalar == "\r" || scalar.properties.isWhitespace
    }

    private static func isPunctuation(_ scalar: Unicode.Scalar) -> Bool {
        if scalar.isASCII, (33...47).contains(scalar.value) || (58...64).contains(scalar.value)
            || (91...96).contains(scalar.value) || (123...126).contains(scalar.value) {
            return true
        }
        switch scalar.properties.generalCategory {
        case .connectorPunctuation, .dashPunctuation, .openPunctuation, .closePunctuation,
             .initialPunctuation, .finalPunctuation, .otherPunctuation:
            return true
        default:
            return false
        }
    }

    private static func isChinese(_ scalar: Unicode.Scalar) -> Bool {
        switch scalar.value {
        case 0x4E00...0x9FFF, 0x3400...0x4DBF, 0x20000...0x2A6DF, 0x2A700...0x2B73F,
             0x2B740...0x2B81F, 0x2B920...0x2CEAF, 0xF900...0xFAFF, 0x2F800...0x2FA1F:
            return true
        default:
            return false
        }
    }

    private func basicTokenize(_ input: String) -> [BasicToken] {
        var tokens: [BasicToken] = []
        var current: [(Unicode.Scalar, Int, Int)] = []

        func flush() {
            if current.isEmpty { return }
            tokens.append((String(String.UnicodeScalarView(current.map(\.0))), current))
            current.removeAll()
        }

        for (offset, character) in input.enumerated() {
            for raw in character.unicodeScalars {
                if raw.value == 0 || raw.value == 0xFFFD || Self.isControl(raw) {
                    continue
                }
                if Self.isWhitespace(raw) {
                    flush()
                    continue
                }
                if Self.isChinese(raw) {
                    flush()
                    tokens.append((String(raw), [(raw, offset, offset + 1)]))
                    continue
                }
                let decomposed = String(raw).decomposedStringWithCanonicalMapping.unicodeScalars
                for base in decomposed where base.properties.generalCategory != .nonspacingMark {
                    for scalar in String(base).lowercased().unicodeScalars {
                        if Self.isPunctuation(scalar) {
                            flush()
                            tokens.append((String(scalar), [(scalar, offset, offset + 1)]))
                        } else {
                            current.append((scalar, offset, offset + 1))
                        }
                    }
                }
            }
        }
        flush()
        return tokens
    }

    private func encodeWordPiece(_ token: BasicToken) -> [(Int32, Int, Int)] {
        if token.1.count > Self.maxInputCharsPerWord {
            return [(unkId, token.1.first?.1 ?? 0, token.1.last?.2 ?? 0)]
        }
        if let id = vocab[token.0] {
            return [(id, token.1.first?.1 ?? 0, token.1.last?.2 ?? 0)]
        }

        let scalars = token.1.map(\.0)
        var start = 0
        var pieces: [(Int32, Int, Int)] = []
        while start < scalars.count {
            var end = scalars.count
            var best: (String, Int)? = nil
            while end > start {
                let fragment = String(String.UnicodeScalarView(scalars[start..<end]))
                let candidate = start == 0 ? fragment : "##\(fragment)"
                if vocab[candidate] != nil {
                    best = (candidate, end)
                    break
                }
                end -= 1
            }
            guard let (piece, bestEnd) = best, let id = vocab[piece] else {
                return [(unkId, token.1.first?.1 ?? 0, token.1.last?.2 ?? 0)]
            }
            pieces.append((id, token.1[start].1, token.1[bestEnd - 1].2))
            start = bestEnd
        }
        return pieces
    }
}

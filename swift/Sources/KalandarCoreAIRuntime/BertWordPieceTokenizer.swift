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

    private func basicTokenize(_ input: String) -> [(String, [(Character, Int, Int)])] {
        var tokens: [(String, [(Character, Int, Int)])] = []
        var current: [(Character, Int, Int)] = []

        func flush() {
            if current.isEmpty { return }
            tokens.append((String(current.map(\.0)), current))
            current.removeAll()
        }

        let lower = input.lowercased()
        for (offset, char) in lower.enumerated() {
            if char.isWhitespace {
                flush()
                continue
            }
            if char.isPunctuation || char.isSymbol {
                flush()
                tokens.append((String(char), [(char, offset, offset + 1)]))
                continue
            }
            current.append((char, offset, offset + 1))
        }
        flush()
        return tokens
    }

    private func encodeWordPiece(_ token: (String, [(Character, Int, Int)])) -> [(Int32, Int, Int)] {
        if let id = vocab[token.0] {
            return [(id, token.1.first?.1 ?? 0, token.1.last?.2 ?? 0)]
        }

        let chars = Array(token.0)
        var start = 0
        var pieces: [(Int32, Int, Int)] = []
        while start < chars.count {
            var end = chars.count
            var best: (String, Int)? = nil
            while end > start {
                let fragment = String(chars[start..<end])
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

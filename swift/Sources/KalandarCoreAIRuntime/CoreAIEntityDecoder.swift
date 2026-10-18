import Foundation
import KalandarDates

/// A contiguous source span emitted by a token-classification Core AI model.
public struct CoreAIEntity: Equatable, Sendable {
    public let field: String
    public let text: String
    public let startOffset: Int
    public let endOffset: Int
    public let confidence: Double

    public init(field: String, text: String, startOffset: Int, endOffset: Int, confidence: Double) {
        self.field = field
        self.text = text
        self.startOffset = startOffset
        self.endOffset = endOffset
        self.confidence = confidence
    }
}

/// First-piece decoding strategy declared by a token-classification artifact.
public enum CoreAISequenceDecoding: String, Sendable, Equatable {
    case argmax
    case bio
}

/// First-piece sequence decoding from the round-two tensor contract: whitespace
/// words keep only their first subword piece, the `[words, labels]` logits are
/// decoded with argmax or constrained BIO Viterbi, and spans are rebuilt from
/// whitespace word offsets.
public enum CoreAISequenceDecoder {
    public struct Word: Equatable, Sendable {
        public let start: Int
        public let end: Int

        public init(start: Int, end: Int) {
            self.start = start
            self.end = end
        }
    }

    public struct Span: Equatable, Sendable {
        public let field: String
        public let start: Int
        public let end: Int
    }

    /// Splits on Python `str.split()` whitespace, returning character offsets.
    public static func whitespaceWords(_ characters: [Character]) -> [Word] {
        var words: [Word] = []
        var start: Int?
        for (index, character) in characters.enumerated() {
            if isPythonWhitespace(character) {
                if let wordStart = start {
                    words.append(Word(start: wordStart, end: index))
                    start = nil
                }
            } else if start == nil {
                start = index
            }
        }
        if let wordStart = start {
            words.append(Word(start: wordStart, end: characters.count))
        }
        return words
    }

    /// Maps each word to the token index of its first subword piece, or nil when truncated.
    public static func alignFirstPieces(words: [Word], offsets: [(Int, Int)]) -> [Int?] {
        var firstPieces = [Int?](repeating: nil, count: words.count)
        var wordIndex = 0
        for (tokenIndex, offset) in offsets.enumerated() {
            let (start, end) = offset
            guard end > start else { continue }
            while wordIndex < words.count, words[wordIndex].end <= start {
                wordIndex += 1
            }
            guard wordIndex < words.count else { break }
            if start >= words[wordIndex].start, firstPieces[wordIndex] == nil {
                firstPieces[wordIndex] = tokenIndex
            }
        }
        return firstPieces
    }

    public static func validateLabels(_ labels: [String]) throws {
        guard Set(labels).count == labels.count, labels.contains("O") else {
            throw KalandarCoreAIError.invalidManifest("labels must be unique BIO labels including O")
        }
        for label in labels where label != "O" {
            guard label.count > 2, label.hasPrefix("B-") || label.hasPrefix("I-") else {
                throw KalandarCoreAIError.invalidManifest("invalid BIO label \(label)")
            }
            if label.hasPrefix("I-"), !labels.contains("B-" + label.dropFirst(2)) {
                throw KalandarCoreAIError.invalidManifest("label \(label) requires a matching B label")
            }
        }
    }

    public static func startAllowed(_ labels: [String]) -> [Bool] {
        labels.map { !$0.hasPrefix("I-") }
    }

    public static func transitionAllowed(_ labels: [String]) -> [[Bool]] {
        labels.map { previous in
            labels.map { next in
                !next.hasPrefix("I-") || previous == next || previous == "B-" + next.dropFirst(2)
            }
        }
    }

    public static func decodeArgmax(_ logits: [[Double]], labels: [String]) throws -> [Int] {
        try validate(logits, labels: labels)
        return logits.map(argmaxIndex)
    }

    /// Constrained BIO Viterbi in Double precision. Exact ties choose the lowest
    /// predecessor id, then the lowest final id.
    public static func decodeBioViterbi(_ logits: [[Double]], labels: [String]) throws -> [Int] {
        try validate(logits, labels: labels)
        guard let first = logits.first else { return [] }
        let labelCount = labels.count
        let starts = startAllowed(labels)
        let allowed = transitionAllowed(labels)
        var scores = (0..<labelCount).map { starts[$0] ? first[$0] : -Double.infinity }
        var backpointers: [[Int]] = []

        for row in logits.dropFirst() {
            var next = [Double](repeating: 0, count: labelCount)
            var pointers = [Int](repeating: 0, count: labelCount)
            for label in 0..<labelCount {
                var best = -Double.infinity
                var bestPrevious = 0
                for previous in 0..<labelCount where allowed[previous][label] && scores[previous] > best {
                    best = scores[previous]
                    bestPrevious = previous
                }
                next[label] = row[label] + best
                pointers[label] = bestPrevious
            }
            backpointers.append(pointers)
            scores = next
        }

        var current = argmaxIndex(scores)
        var path = [current]
        for pointers in backpointers.reversed() {
            current = pointers[current]
            path.append(current)
        }
        return path.reversed()
    }

    public static func decode(_ logits: [[Double]], labels: [String], decoding: CoreAISequenceDecoding) throws -> [Int] {
        switch decoding {
        case .argmax: try decodeArgmax(logits, labels: labels)
        case .bio: try decodeBioViterbi(logits, labels: labels)
        }
    }

    /// O ends a span, B always starts one, and I extends the current span only
    /// when the previous word carries the same field.
    public static func rebuildSpans(words: [Word], wordLabels: [String]) -> [Span] {
        var spans: [Span] = []
        for (index, word) in words.enumerated() {
            let label = index < wordLabels.count ? wordLabels[index] : "O"
            guard label != "O" else { continue }
            let field = String(label.dropFirst(2))
            let previous = index > 0 && index - 1 < wordLabels.count ? wordLabels[index - 1] : "O"
            if label.hasPrefix("I-"), previous != "O", previous.dropFirst(2) == field, let last = spans.last {
                spans[spans.count - 1] = Span(field: last.field, start: last.start, end: word.end)
            } else {
                spans.append(Span(field: field, start: word.start, end: word.end))
            }
        }
        return spans
    }

    /// Checks that a `sequence_decoder.json` sidecar declares exactly these BIO rules.
    public static func validateSidecar(_ data: Data, labels: [String]) throws {
        struct Sidecar: Decodable {
            var algorithm: String
            var first_subword_piece_only: Bool
            var accumulator: String
            var start_scores: [Double?]
            var transition_scores_previous_by_next: [[Double?]]
            var end_scores: [Double?]
            var null_score_means: String
            var tie_break: String
            var labels: [String]
        }
        let sidecar = try JSONDecoder().decode(Sidecar.self, from: data)
        try validateLabels(labels)
        let legal: (Bool) -> Double? = { $0 ? 0 : nil }
        let valid = sidecar.algorithm == "BIO Viterbi"
            && sidecar.first_subword_piece_only
            && sidecar.accumulator == "float64"
            && sidecar.null_score_means == "negative infinity"
            && sidecar.tie_break == "lowest predecessor ID, then lowest final ID"
            && sidecar.labels == labels
            && sidecar.start_scores == startAllowed(labels).map(legal)
            && sidecar.transition_scores_previous_by_next == transitionAllowed(labels).map { $0.map(legal) }
            && sidecar.end_scores == labels.map { _ in 0 }
        guard valid else {
            throw KalandarCoreAIError.invalidManifest("sequence_decoder.json does not match the BIO decoding contract")
        }
    }

    static func argmaxIndex(_ row: [Double]) -> Int {
        var best = 0
        for index in row.indices where row[index] > row[best] {
            best = index
        }
        return best
    }

    private static func validate(_ logits: [[Double]], labels: [String]) throws {
        try validateLabels(labels)
        guard logits.allSatisfy({ $0.count == labels.count && $0.allSatisfy(\.isFinite) }) else {
            throw KalandarCoreAIError.invalidModelOutput("sequence decoder requires finite [words, \(labels.count)] logits")
        }
    }

    private static func isPythonWhitespace(_ character: Character) -> Bool {
        character.isWhitespace || character.unicodeScalars.allSatisfy { (0x1C...0x1F).contains($0.value) }
    }
}

public enum CoreAIEntityDecoder {
    /// Orders a manifest label map by contiguous id.
    public static func orderedLabels(_ labels: [String: String]) throws -> [String] {
        guard !labels.isEmpty else {
            throw KalandarCoreAIError.invalidManifest("token-classification artifact has no labels")
        }
        return try (0..<labels.count).map { id in
            guard let label = labels[String(id)] else {
                throw KalandarCoreAIError.invalidManifest("token-classification labels are not contiguous: missing id \(id)")
            }
            return label
        }
    }

    /// Decodes the first subword piece of each whitespace word with argmax or
    /// BIO Viterbi and maps the spans back onto the original capture text.
    public static func decode(
        output: CoreAITensorOutput,
        tokenized: TokenizedText,
        labels: [String: String],
        sourceText: String,
        decoding: CoreAISequenceDecoding = .argmax
    ) throws -> [CoreAIEntity] {
        let orderedLabels = try orderedLabels(labels)
        let labelCount = orderedLabels.count
        let tokenCount = tokenized.inputIds.count
        guard tokenized.attentionMask.count == tokenCount, tokenized.offsets.count == tokenCount else {
            throw KalandarCoreAIError.invalidModelOutput("tokenizer output has inconsistent lengths")
        }
        guard output.shape == [1, tokenCount, labelCount] || output.shape == [tokenCount, labelCount] else {
            throw KalandarCoreAIError.invalidModelOutput(
                "expected logits shaped [1, \(tokenCount), \(labelCount)] or [\(tokenCount), \(labelCount)], received \(output.shape)"
            )
        }
        guard output.values.count == tokenCount * labelCount else {
            throw KalandarCoreAIError.invalidModelOutput(
                "expected \(tokenCount * labelCount) logits, received \(output.values.count)"
            )
        }

        let sourceCharacters = Array(sourceText)
        let maskedOffsets = zip(tokenized.offsets, tokenized.attentionMask).map { offset, mask in
            mask != 0 && offset.0 >= 0 && offset.1 <= sourceCharacters.count ? offset : (0, 0)
        }
        let words = CoreAISequenceDecoder.whitespaceWords(sourceCharacters)
        let firstPieces = CoreAISequenceDecoder.alignFirstPieces(words: words, offsets: maskedOffsets)
        let represented = firstPieces.enumerated().compactMap { wordIndex, tokenIndex in
            tokenIndex.map { (wordIndex, $0) }
        }
        let rows = represented.map { _, tokenIndex in
            (0..<labelCount).map { Double(output.values[tokenIndex * labelCount + $0]) }
        }
        let labelIDs = try CoreAISequenceDecoder.decode(rows, labels: orderedLabels, decoding: decoding)

        var wordLabels = [String](repeating: "O", count: words.count)
        var wordConfidences = [Double](repeating: 0, count: words.count)
        for ((wordIndex, _), (row, labelID)) in zip(represented, zip(rows, labelIDs)) {
            wordLabels[wordIndex] = orderedLabels[labelID]
            wordConfidences[wordIndex] = softmaxProbability(row, selected: labelID)
        }

        return CoreAISequenceDecoder.rebuildSpans(words: words, wordLabels: wordLabels).compactMap { span in
            let text = String(sourceCharacters[span.start..<span.end])
                .trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return nil }
            let spanWords = words.indices.filter { words[$0].start >= span.start && words[$0].end <= span.end }
            let confidence = spanWords.map { wordConfidences[$0] }.reduce(0, +) / Double(max(spanWords.count, 1))
            return CoreAIEntity(
                field: span.field,
                text: text,
                startOffset: span.start,
                endOffset: span.end,
                confidence: confidence
            )
        }
    }

    private static func softmaxProbability(_ row: [Double], selected: Int) -> Double {
        guard let maxLogit = row.max() else { return 0 }
        let normalizer = row.reduce(0.0) { $0 + Foundation.exp($1 - maxLogit) }
        return normalizer == 0 ? 0 : Foundation.exp(row[selected] - maxLogit) / normalizer
    }
}

enum CoreAIEntityMerger {
    static func apply(_ entities: [CoreAIEntity], to base: CapturePreviewResult) -> CapturePreviewResult {
        var result = base
        let grouped = Dictionary(grouping: entities, by: \.field)

        func first(_ field: String) -> String? {
            grouped[field]?.first?.text
        }

        func all(_ field: String) -> [String] {
            (grouped[field] ?? []).map(\.text)
        }

        switch result.kind {
        case "task":
            guard var task = result.task else { return result }
            task.title = first("title") ?? task.title
            task.assignees = normalizedValues(all("assignee"), removing: "@", fallback: task.assignees)
            task.tags = normalizedValues(all("tags"), removing: "#", fallback: task.tags)
            task.projectHint = first("project") ?? task.projectHint
            task.priority = first("priority") ?? task.priority
            task.status = first("status") ?? task.status
            task.dueDateText = first("due_date") ?? task.dueDateText
            task.dueTimeText = first("due_time") ?? task.dueTimeText
            task.durationText = first("duration") ?? task.durationText
            task.recurrenceText = first("recurrence") ?? task.recurrenceText
            task.recurrenceStartText = first("recurrence_start") ?? task.recurrenceStartText
            task.recurrenceEndText = first("recurrence_end") ?? task.recurrenceEndText
            task.recurrenceExceptions = mergedExceptions(all("recurrence_exception"), existing: task.recurrenceExceptions)
            result.task = task

        case "reminder":
            guard var reminder = result.reminder else { return result }
            reminder.title = first("title") ?? reminder.title
            reminder.remindAtText = first("trigger_time") ?? reminder.remindAtText
            reminder.leadTimeText = first("lead_time") ?? reminder.leadTimeText
            reminder.recurrenceStartText = first("recurrence_start") ?? reminder.recurrenceStartText
            reminder.recurrenceEndText = first("recurrence_end") ?? reminder.recurrenceEndText
            reminder.recurrenceWindowText = first("recurrence_window") ?? reminder.recurrenceWindowText
            reminder.location = first("location") ?? reminder.location
            reminder.recurrenceExceptions = mergedExceptions(all("recurrence_exception"), existing: reminder.recurrenceExceptions)
            result.reminder = reminder

        case "calendar-event":
            guard var event = result.event else { return result }
            event.title = first("title") ?? event.title
            event.descriptionText = first("description") ?? event.descriptionText
            event.timeText = first("start_time") ?? event.timeText
            event.endTimeText = first("end_time") ?? event.endTimeText
            event.location = first("location") ?? event.location
            event.recurrenceText = first("recurrence") ?? event.recurrenceText
            event.recurrenceStartText = first("recurrence_start") ?? event.recurrenceStartText
            event.recurrenceEndText = first("recurrence_end") ?? event.recurrenceEndText
            event.recurrenceExceptions = mergedExceptions(all("recurrence_exception"), existing: event.recurrenceExceptions)
            result.event = event

        default:
            break
        }

        return result
    }

    static func resolveSchedule(
        _ base: CapturePreviewResult,
        entities: [CoreAIEntity],
        input: String,
        request: CapturePreviewRequest
    ) -> CapturePreviewResult {
        let dateFields: Set<String> = [
            "due_date", "due_time", "duration", "trigger_time", "lead_time", "start_time", "end_time",
            "recurrence", "recurrence_start", "recurrence_end", "recurrence_exception", "recurrence_window",
        ]
        let kind: String
        switch base.kind {
        case "task": kind = "task"
        case "reminder": kind = "reminder"
        case "calendar-event": kind = "calendar"
        default: return base
        }
        let labels = entities.filter { dateFields.contains($0.field) }.compactMap { entity -> ScheduleLabel? in
            let characters = input.count
            guard entity.startOffset >= 0, entity.endOffset <= characters, entity.startOffset < entity.endOffset else {
                return nil
            }
            let start = input.index(input.startIndex, offsetBy: entity.startOffset)
            let end = input.index(input.startIndex, offsetBy: entity.endOffset)
            return ScheduleLabel(
                label: entity.field,
                start: input.utf16.distance(from: input.utf16.startIndex, to: start),
                end: input.utf16.distance(from: input.utf16.startIndex, to: end)
            )
        }
        guard !labels.isEmpty else { return base }
        let schedule = ScheduleExtraction(input, kind: kind, request: request, labels: labels)
        var result = base
        result.schedule = schedule.schedule
        result.warnings = result.warnings.filter { $0.severity != "info" } + schedule.warnings
        if var reminder = result.reminder {
            reminder.recurrence = schedule.reminderRecurrence ?? reminder.recurrence
            result.reminder = reminder
        }
        if var event = result.event {
            event.durationMinutes = schedule.schedule?.durationMinutes ?? event.durationMinutes
            let offsets = schedule.alertOffsets
            event.alertOffsetMinutes = offsets.isEmpty ? event.alertOffsetMinutes : offsets
            result.event = event
        }
        return result
    }

    private static func normalizedValues(_ values: [String], removing prefix: Character, fallback: [String]) -> [String] {
        let normalized = values.flatMap { value in
            value
                .split(separator: ",")
                .map { TextUtilities.shadow(String($0)).trimmingCharacters(in: CharacterSet(charactersIn: String(prefix))) }
                .filter { !$0.isEmpty }
        }
        return normalized.isEmpty ? fallback : Array(Set(normalized)).sorted()
    }

    private static func mergedExceptions(
        _ values: [String],
        existing: [CapturePreviewRecurrenceException]
    ) -> [CapturePreviewRecurrenceException] {
        let extras = values.map { CapturePreviewRecurrenceException(raw: $0) }
        var seen = Set<String>()
        return (existing + extras).filter { seen.insert($0.raw).inserted }
    }


}

import Foundation

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

public enum CoreAIEntityDecoder {
    /// Decodes BIO token labels and maps them back onto the original capture text.
    public static func decode(
        output: CoreAITensorOutput,
        tokenized: TokenizedText,
        labels: [String: String],
        sourceText: String
    ) throws -> [CoreAIEntity] {
        guard let highestLabelID = labels.keys.compactMap({ Int($0) }).max() else {
            throw KalandarCoreAIError.invalidManifest("token-classification artifact has no labels")
        }
        let labelCount = highestLabelID + 1
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

        struct PendingEntity {
            var field: String
            var startOffset: Int
            var endOffset: Int
            var confidenceSum: Double
            var tokenCount: Int
        }

        let sourceCharacters = Array(sourceText)
        var pending: [PendingEntity] = []
        var previousField: String?

        for tokenIndex in 0..<tokenCount {
            guard tokenized.attentionMask[tokenIndex] != 0 else {
                previousField = nil
                continue
            }
            let (startOffset, endOffset) = tokenized.offsets[tokenIndex]
            guard startOffset >= 0, endOffset > startOffset, endOffset <= sourceCharacters.count else {
                previousField = nil
                continue
            }

            let logitsStart = tokenIndex * labelCount
            var bestLabelID = 0
            var maxLogit = -Double.greatestFiniteMagnitude
            for labelID in 0..<labelCount {
                let value = Double(output.values[logitsStart + labelID])
                if value > maxLogit {
                    maxLogit = value
                    bestLabelID = labelID
                }
            }

            let normalizer = (0..<labelCount).reduce(0.0) { partial, labelID in
                partial + Foundation.exp(Double(output.values[logitsStart + labelID]) - maxLogit)
            }
            let confidence = normalizer == 0 ? 0 : 1 / normalizer
            guard let label = labels[String(bestLabelID)], let parsed = parseBioLabel(label) else {
                previousField = nil
                continue
            }

            if parsed.prefix == "I",
               let lastIndex = pending.indices.last,
               pending[lastIndex].field == parsed.field,
               previousField == parsed.field {
                pending[lastIndex].endOffset = max(pending[lastIndex].endOffset, endOffset)
                pending[lastIndex].confidenceSum += confidence
                pending[lastIndex].tokenCount += 1
            } else {
                pending.append(PendingEntity(
                    field: parsed.field,
                    startOffset: startOffset,
                    endOffset: endOffset,
                    confidenceSum: confidence,
                    tokenCount: 1
                ))
            }
            previousField = parsed.field
        }

        return pending.compactMap { entity in
            let text = String(sourceCharacters[entity.startOffset..<entity.endOffset])
                .trimmingCharacters(in: .whitespacesAndNewlines)
            guard !text.isEmpty else { return nil }
            return CoreAIEntity(
                field: entity.field,
                text: text,
                startOffset: entity.startOffset,
                endOffset: entity.endOffset,
                confidence: entity.confidenceSum / Double(entity.tokenCount)
            )
        }
    }

    private static func parseBioLabel(_ label: String) -> (prefix: String, field: String)? {
        let pieces = label.split(separator: "-", maxSplits: 1).map(String.init)
        guard pieces.count == 2, (pieces[0] == "B" || pieces[0] == "I"), !pieces[1].isEmpty else {
            return nil
        }
        return (pieces[0], pieces[1])
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
            reminder.recurrence = recurrence(first("recurrence")) ?? reminder.recurrence
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
            event.durationMinutes = durationMinutes(first("duration")) ?? event.durationMinutes
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

    private static func recurrence(_ raw: String?) -> ReminderRecurrence? {
        guard let raw else { return nil }
        let value = TextUtilities.shadow(raw)
        let interval = value.contains("every other") ? 2 : 1
        if value.contains("daily") || value.contains("every day") {
            return ReminderRecurrence(frequency: "daily", interval: interval)
        }
        let days = TextUtilities.weekdays.filter { value.contains($0) }
        if value.contains("week") || !days.isEmpty {
            return ReminderRecurrence(frequency: "weekly", interval: interval, days: days.isEmpty ? nil : days)
        }
        if value.contains("month") {
            return ReminderRecurrence(frequency: "monthly", interval: interval)
        }
        if value.contains("year") || value.contains("annual") {
            return ReminderRecurrence(frequency: "yearly", interval: interval)
        }
        return nil
    }

    private static func durationMinutes(_ raw: String?) -> Int? {
        guard let raw else { return nil }
        let value = TextUtilities.shadow(raw)
        let number = value.split(whereSeparator: { !$0.isNumber }).first.flatMap { Int($0) }
        guard let number else { return nil }
        if value.contains("hour") || value.contains("hr") {
            return number * 60
        }
        return value.contains("min") ? number : nil
    }
}

import Foundation
import KalandarDates

public struct LightweightCaptureParser: Sendable {
    public init() {}

    public func parse(_ request: CapturePreviewRequest) -> CapturePreviewResult {
        do {
            let payload = String(decoding: try JSONEncoder().encode(request), as: UTF8.self)
            let response = Data(KalandarDates.capturePreviewJSON(payload).utf8)
            if let failure = try? JSONDecoder().decode(RulesFailure.self, from: response) {
                return unsupported("capture-preview-failed", failure.error)
            }
            return try JSONDecoder().decode(CapturePreviewResult.self, from: response)
        } catch {
            return unsupported("capture-preview-failed", String(describing: error))
        }
    }

    private func unsupported(_ code: String, _ message: String) -> CapturePreviewResult {
        CapturePreviewResult(
            kind: "unsupported",
            confidence: 0,
            unavailableReason: code,
            warnings: [CapturePreviewWarning(code: code, severity: "warning", message: message)]
        )
    }
}

private struct RulesFailure: Decodable {
    var error: String
}

struct ScheduleExtraction {
    let remaining: String
    let texts: ScheduleTexts
    let schedule: PreviewSchedule?
    let warnings: [CapturePreviewWarning]

    init(_ text: String, kind: String, request: CapturePreviewRequest, labels: [ScheduleLabel]? = nil) {
        let found = try? KalandarDates.captureSchedule(
            text: text,
            kind: kind,
            nowISO: request.nowIso,
            timezone: request.timezone,
            locale: request.locale,
            spans: labels
        )
        if let found {
            remaining = KalandarDates.removeSpans(found.spans, from: text)
            texts = found.texts
            schedule = found.schedule
            warnings = found.warnings.map { CapturePreviewWarning(code: $0.code, severity: "info", message: $0.message) }
        } else {
            remaining = text
            texts = .empty
            schedule = nil
            warnings = []
        }
    }

    var recurrenceExceptions: [CapturePreviewRecurrenceException] {
        let resolved = schedule?.recurrence?.exceptions ?? []
        let sameCount = resolved.count == texts.recurrenceExceptions.count
        return texts.recurrenceExceptions.enumerated().map { index, raw in
            let matched = sameCount
                ? resolved[index]
                : resolved.first { $0.raw.contains(raw) || raw.contains($0.raw) }
            return CapturePreviewRecurrenceException(raw: raw, condition: matched?.condition, action: matched?.action)
        }
    }

    var reminderRecurrence: ReminderRecurrence? {
        guard let recurrence = schedule?.recurrence else { return nil }
        let unpositioned = recurrence.weekdays.filter { $0.position == nil }.map(\.weekday)
        let days = recurrence.days ?? (unpositioned.isEmpty ? nil : unpositioned)
        return ReminderRecurrence(frequency: recurrence.frequency, interval: recurrence.interval, days: days)
    }

    var clockTime: String? {
        guard let time = texts.time else { return nil }
        let lower = time.lowercased()
        return time.contains(where: \.isNumber) || lower == "noon" || lower == "midnight" ? time : nil
    }

    var contextHint: String? {
        for text in [texts.moment, texts.time, texts.recurrence].compactMap({ $0 }) {
            if let match = TextUtilities.firstMatch("\\b(end of (?:the )?(?:month|week|day)|next month|morning|noon|afternoon|evening|night)\\b", in: text),
               let value = TextUtilities.group(0, from: match, in: text) {
                return value.lowercased().replacingOccurrences(of: "end of the ", with: "end of ")
            }
        }
        return nil
    }

    var alertOffsets: [Int] {
        guard let minutes = schedule?.alertMinutesBefore ?? schedule?.leadMinutes, minutes >= 0 else { return [] }
        return [minutes]
    }
}

import Foundation
import KalandarDates

public struct LightweightCaptureParser: Sendable {
    public init() {}

    public func parse(_ request: CapturePreviewRequest) -> CapturePreviewResult {
        let text = TextUtilities.shadow(request.text)
        if text.isEmpty {
            return unsupported("capture-text-required", "Capture preview requires non-empty text.")
        }

        if let payload = TextUtilities.stripPrefix(text, prefix: "task:") {
            return payload.isEmpty ? unsupported("capture-text-required", "Capture preview requires non-empty text.") : taskResult(payload, request, 0.99, [])
        }
        if let payload = TextUtilities.stripPrefix(text, prefix: "reminder:") {
            return payload.isEmpty ? unsupported("capture-text-required", "Capture preview requires non-empty text.") : reminderResult(payload, request, 0.99, [])
        }
        if let payload = TextUtilities.stripPrefix(text, prefix: "event:") {
            return payload.isEmpty ? unsupported("capture-text-required", "Capture preview requires non-empty text.") : calendarResult(payload, request, 0.99, [])
        }

        let route = IntentRouter.route(text)
        let candidates = route.candidates
        switch route.kind {
        case "task":
            return taskResult(text, request, route.confidence, candidates)
        case "reminder":
            return reminderResult(text, request, route.confidence, candidates)
        case "calendar-event":
            return calendarResult(text, request, route.confidence, candidates)
        default:
            return CapturePreviewResult(
                kind: "thought",
                confidence: route.confidence,
                candidates: candidates,
                thought: CapturePreviewThoughtDraft(text: text)
            )
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

    private func taskResult(
        _ text: String,
        _ request: CapturePreviewRequest,
        _ confidence: Double,
        _ candidates: [CapturePreviewCandidate]
    ) -> CapturePreviewResult {
        let parsed = TaskLightweightParser.parse(text, request: request)
        return CapturePreviewResult(
            kind: "task",
            confidence: confidence,
            candidates: candidates,
            task: parsed.draft,
            warnings: parsed.schedule.warnings,
            schedule: parsed.schedule.schedule
        )
    }

    private func reminderResult(
        _ text: String,
        _ request: CapturePreviewRequest,
        _ confidence: Double,
        _ candidates: [CapturePreviewCandidate]
    ) -> CapturePreviewResult {
        let parsed = ReminderLightweightParser.parse(text, request: request)
        return CapturePreviewResult(
            kind: "reminder",
            confidence: confidence,
            candidates: candidates,
            reminder: parsed.draft,
            warnings: parsed.schedule.warnings,
            schedule: parsed.schedule.schedule
        )
    }

    private func calendarResult(
        _ text: String,
        _ request: CapturePreviewRequest,
        _ confidence: Double,
        _ candidates: [CapturePreviewCandidate]
    ) -> CapturePreviewResult {
        let parsed = CalendarLightweightParser.parse(text, request: request)
        return CapturePreviewResult(
            kind: "calendar-event",
            confidence: confidence,
            candidates: candidates,
            event: parsed.draft,
            warnings: parsed.schedule.warnings,
            schedule: parsed.schedule.schedule
        )
    }
}

struct IntentRoute {
    var kind: String
    var confidence: Double
    var candidates: [CapturePreviewCandidate]
}

enum IntentRouter {
    static func route(_ text: String) -> IntentRoute {
        let kind: String
        if text.hasPrefix("remind me") || text.hasPrefix("don't forget") || text.hasPrefix("dont forget") {
            kind = "reminder"
        } else if text.hasPrefix("idea ") || text.hasPrefix("idea about") || text.hasPrefix("note ") {
            kind = "thought"
        } else if text.contains("meeting") || text.contains("lunch with") || text.contains("design review")
                    || text.contains(" at night") || text.contains("daily digest tomorrow")
                    || text.contains(" every other tuesday") {
            kind = "calendar-event"
        } else if text.contains("tomorrow") || text.contains("end of month") || text.contains("project ") {
            kind = "task"
        } else {
            kind = "thought"
        }

        let scores: [CapturePreviewCandidate]
        switch kind {
        case "task":
            scores = [
                CapturePreviewCandidate(kind: "task", score: 0.9),
                CapturePreviewCandidate(kind: "calendar-event", score: 0.25),
                CapturePreviewCandidate(kind: "reminder", score: 0.2),
            ]
        case "reminder":
            scores = [
                CapturePreviewCandidate(kind: "reminder", score: 0.92),
                CapturePreviewCandidate(kind: "task", score: 0.35),
                CapturePreviewCandidate(kind: "calendar-event", score: 0.25),
            ]
        case "calendar-event":
            scores = [
                CapturePreviewCandidate(kind: "calendar-event", score: 0.92),
                CapturePreviewCandidate(kind: "reminder", score: 0.32),
                CapturePreviewCandidate(kind: "task", score: 0.25),
            ]
        default:
            scores = []
        }

        return IntentRoute(kind: kind, confidence: scores.first?.score ?? 0.8, candidates: scores)
    }
}

enum TaskLightweightParser {
    static func parse(_ input: String, request: CapturePreviewRequest) -> (draft: CapturePreviewTaskDraft, schedule: ScheduleExtraction) {
        var working = TextUtilities.shadow(input)
        let priority = extractFirst(["\\bhigh\\s+priority\\b", "\\burgent\\b"], value: "high", from: &working)
            ?? extractFirst(["\\bmedium\\s+priority\\b"], value: "medium", from: &working)
            ?? extractFirst(["\\blow\\s+priority\\b"], value: "low", from: &working)
        let status = extractStatus(from: &working)
        let tags = extractTags(from: &working)
        let project = extractProject(from: &working)
        let assignees = extractAssignees(from: &working, knownAssignees: request.knownAssignees)
        let schedule = ScheduleExtraction(working, kind: "task", request: request)
        let title = TextUtilities.cleanTitle(schedule.remaining, fallback: "task")

        let draft = CapturePreviewTaskDraft(
            title: title,
            assignees: assignees,
            projectHint: project,
            tags: tags,
            priority: priority,
            status: status,
            dueDateText: schedule.texts.date,
            dueTimeText: schedule.texts.time,
            durationText: schedule.texts.duration,
            recurrenceText: schedule.texts.recurrence,
            recurrenceStartText: schedule.texts.recurrenceStart,
            recurrenceEndText: schedule.texts.recurrenceEnd,
            recurrenceExceptions: schedule.recurrenceExceptions
        )
        return (draft, schedule)
    }

    private static func extractFirst(_ patterns: [String], value: String, from working: inout String) -> String? {
        for pattern in patterns {
            if let match = TextUtilities.firstMatch(pattern, in: working) {
                TextUtilities.removeMatch(match, from: &working)
                return value
            }
        }
        return nil
    }

    private static func extractStatus(from working: inout String) -> String? {
        for (pattern, value) in [
            ("\\bin\\s+progress\\b", "in progress"),
            ("\\bnot\\s+started\\b", "not started"),
            ("\\bblocked\\b", "blocked"),
            ("\\bwaiting\\b", "waiting"),
            ("\\bdone\\b|\\bcompleted?\\b", "done"),
        ] {
            if let match = TextUtilities.firstMatch(pattern, in: working) {
                TextUtilities.removeMatch(match, from: &working)
                return value
            }
        }
        return nil
    }


    private static func extractTags(from working: inout String) -> [String] {
        var tags: [String] = []
        if let match = TextUtilities.firstMatch("\\btags?\\s*:?\\s*([^.;]+)$", in: working),
           let raw = TextUtilities.group(1, from: match, in: working) {
            tags.append(contentsOf: raw.split(whereSeparator: { ", ".contains($0) }).map(String.init))
            TextUtilities.removeMatch(match, from: &working)
        }

        let hashtagMatches = TextUtilities.matches("#[a-z0-9][a-z0-9_-]*", in: working)
        for match in hashtagMatches {
            if let value = TextUtilities.group(0, from: match, in: working) {
                tags.append(value.replacingOccurrences(of: "#", with: ""))
            }
        }
        working = TextUtilities.replacingMatches("#[a-z0-9][a-z0-9_-]*", in: working)
        tags = tags.map { $0.trimmingCharacters(in: CharacterSet(charactersIn: "#,;:!? ")) }
            .filter { !$0.isEmpty }
        return TextUtilities.uniqueSorted(tags)
    }

    private static func extractProject(from working: inout String) -> String? {
        let pattern = "\\b(?:in|for)\\s+(project\\s+[a-z0-9][a-z0-9 -]*?)(?=\\s+(?:with|by|at|high|medium|low|in\\s+progress|blocked|tags?)\\b|[,.;]|$)"
        guard let match = TextUtilities.firstMatch(pattern, in: working),
              let project = TextUtilities.group(1, from: match, in: working) else {
            return nil
        }
        TextUtilities.removeMatch(match, from: &working)
        return TextUtilities.cleanTitle(project, fallback: "project")
    }

    private static func extractAssignees(from working: inout String, knownAssignees: [String]?) -> [String] {
        guard let knownAssignees else { return [] }
        var found: [String] = []
        for assignee in knownAssignees {
            let normalized = TextUtilities.shadow(assignee)
            if TextUtilities.firstMatch("\\b\(NSRegularExpression.escapedPattern(for: normalized))\\b", in: working) != nil {
                found.append(normalized)
            }
        }
        if !found.isEmpty {
            let joined = found.map(NSRegularExpression.escapedPattern).joined(separator: "|")
            working = TextUtilities.replacingMatches("\\bwith\\s+(?:(?:\(joined))(?:\\s+and\\s+(?:\(joined)))*)\\b", in: working)
            working = TextUtilities.replacingMatches("\\b(?:\(joined))\\b", in: working)
        }
        return TextUtilities.uniqueSorted(found)
    }


}

enum ReminderLightweightParser {
    static func parse(_ input: String, request: CapturePreviewRequest) -> (draft: CapturePreviewReminderDraft, schedule: ScheduleExtraction) {
        var working = TextUtilities.shadow(input)
        working = TextUtilities.replacingMatches("^remind\\s+me\\s+(?:to\\s+)?", in: working)
        working = TextUtilities.replacingMatches("^don't\\s+forget\\s+(?:to\\s+)?", in: working)
        working = TextUtilities.replacingMatches("^dont\\s+forget\\s+(?:to\\s+)?", in: working)

        let schedule = ScheduleExtraction(working, kind: "reminder", request: request)
        var remaining = schedule.remaining
        remaining = TextUtilities.replacingMatches("^to\\s+", in: remaining)
        let title = TextUtilities.cleanTitle(remaining, fallback: "reminder")

        let draft = CapturePreviewReminderDraft(
            title: title,
            remindAtText: schedule.texts.moment ?? schedule.clockTime,
            recurrence: schedule.reminderRecurrence,
            recurrenceStartText: schedule.texts.recurrenceStart,
            recurrenceEndText: schedule.texts.recurrenceEnd,
            recurrenceExceptions: schedule.recurrenceExceptions,
            recurrenceWindowText: schedule.texts.recurrenceWindow,
            contextHint: schedule.contextHint
        )
        return (draft, schedule)
    }
}

enum CalendarLightweightParser {
    static func parse(_ input: String, request: CapturePreviewRequest) -> (draft: CapturePreviewCalendarEventDraft, schedule: ScheduleExtraction) {
        let original = TextUtilities.shadow(input)
        let attendees = extractAttendees(from: original)
        let schedule = ScheduleExtraction(original, kind: "calendar", request: request)
        var working = schedule.remaining
        let location = extractLocation(from: &working)
        let title = TextUtilities.cleanTitle(working, fallback: "event")

        let draft = CapturePreviewCalendarEventDraft(
            title: title,
            dateText: schedule.texts.date,
            timeText: schedule.texts.time,
            endTimeText: schedule.texts.endTime,
            durationMinutes: schedule.schedule?.durationMinutes,
            location: location,
            recurrenceText: schedule.texts.recurrence,
            recurrenceStartText: schedule.texts.recurrenceStart,
            recurrenceEndText: schedule.texts.recurrenceEnd,
            recurrenceExceptions: schedule.recurrenceExceptions,
            alertOffsetMinutes: schedule.alertOffsets,
            attendees: attendees
        )
        return (draft, schedule)
    }

    private static func extractLocation(from working: inout String) -> String? {
        guard let match = TextUtilities.firstMatch("\\bat\\s+([a-z][a-z0-9 -]+)$", in: working),
              let value = TextUtilities.group(1, from: match, in: working),
              !value.contains("pm"), !value.contains("am") else {
            return nil
        }
        TextUtilities.removeMatch(match, from: &working)
        return TextUtilities.cleanTitle(value, fallback: "location")
    }



    private static func extractAttendees(from working: String) -> [String] {
        guard let match = TextUtilities.firstMatch("\\bwith\\s+([a-z][a-z0-9-]*)\\b", in: working),
              let value = TextUtilities.group(1, from: match, in: working) else {
            return []
        }
        return [TextUtilities.shadow(value)]
    }
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

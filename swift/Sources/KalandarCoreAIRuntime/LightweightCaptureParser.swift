import Foundation

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
        CapturePreviewResult(
            kind: "task",
            confidence: confidence,
            candidates: candidates,
            task: TaskLightweightParser.parse(text, request: request)
        )
    }

    private func reminderResult(
        _ text: String,
        _ request: CapturePreviewRequest,
        _ confidence: Double,
        _ candidates: [CapturePreviewCandidate]
    ) -> CapturePreviewResult {
        CapturePreviewResult(
            kind: "reminder",
            confidence: confidence,
            candidates: candidates,
            reminder: ReminderLightweightParser.parse(text, request: request)
        )
    }

    private func calendarResult(
        _ text: String,
        _ request: CapturePreviewRequest,
        _ confidence: Double,
        _ candidates: [CapturePreviewCandidate]
    ) -> CapturePreviewResult {
        CapturePreviewResult(
            kind: "calendar-event",
            confidence: confidence,
            candidates: candidates,
            event: CalendarLightweightParser.parse(text, request: request)
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
    static func parse(_ input: String, request: CapturePreviewRequest) -> CapturePreviewTaskDraft {
        var working = applyDateCorrections(TextUtilities.shadow(input))
        let priority = extractFirst(["\\bhigh\\s+priority\\b", "\\burgent\\b"], value: "high", from: &working)
            ?? extractFirst(["\\bmedium\\s+priority\\b"], value: "medium", from: &working)
            ?? extractFirst(["\\blow\\s+priority\\b"], value: "low", from: &working)
        let status = extractStatus(from: &working)
        let duration = extractGroup("\\b(\\d+\\s*(?:minutes?|mins?|hours?|hrs?))\\b", from: &working)
        let tags = extractTags(from: &working)
        let project = extractProject(from: &working)
        let assignees = extractAssignees(from: &working, knownAssignees: request.knownAssignees)
        let dueTime = extractDueTime(from: &working)
        let dueDate = extractDueDate(from: &working)
        let title = TextUtilities.cleanTitle(working, fallback: "task")

        return CapturePreviewTaskDraft(
            title: title,
            assignees: assignees,
            projectHint: project,
            tags: tags,
            priority: priority,
            status: status,
            dueDateText: dueDate,
            dueTimeText: dueTime,
            durationText: duration
        )
    }

    private static func applyDateCorrections(_ input: String) -> String {
        var output = input
        for (from, to) in [
            ("tmrw", "tomorrow"), ("tmr", "tomorrow"), ("tomorow", "tomorrow"),
            ("tonite", "tonight"), ("eod", "end of day"), ("cob", "end of day"),
        ] {
            output = TextUtilities.replacingMatches("\\b\(from)\\b", in: output, with: to)
        }
        return output
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

    private static func extractGroup(_ pattern: String, from working: inout String) -> String? {
        guard let match = TextUtilities.firstMatch(pattern, in: working),
              let value = TextUtilities.group(1, from: match, in: working) else {
            return nil
        }
        TextUtilities.removeMatch(match, from: &working)
        return TextUtilities.shadow(value)
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

    private static func extractDueTime(from working: inout String) -> String? {
        for pattern in ["\\bat\\s+(\\d{1,2}\\s*(?:am|pm))\\b", "\\b(\\d{1,2}\\s*(?:am|pm))\\b"] {
            if let value = extractGroup(pattern, from: &working) {
                return value.replacingOccurrences(of: " ", with: "")
            }
        }
        return nil
    }

    private static func extractDueDate(from working: inout String) -> String? {
        for pattern in [
            "\\b(by\\s+the\\s+first\\s+business\\s+day\\s+next\\s+month)\\b",
            "\\b(tomorrow)\\b",
            "\\b(end\\s+of\\s+month)\\b",
        ] {
            if let value = extractGroup(pattern, from: &working) {
                return value
            }
        }
        return nil
    }
}

enum ReminderLightweightParser {
    static func parse(_ input: String, request _: CapturePreviewRequest) -> CapturePreviewReminderDraft {
        var working = TextUtilities.shadow(input)
        working = TextUtilities.replacingMatches("^remind\\s+me\\s+(?:to\\s+)?", in: working)
        working = TextUtilities.replacingMatches("^don't\\s+forget\\s+(?:to\\s+)?", in: working)
        working = TextUtilities.replacingMatches("^dont\\s+forget\\s+(?:to\\s+)?", in: working)
        let original = working

        let exceptions = extractExceptions(from: &working)
        let recurrenceStart = extractGroup("\\b(starting\\s+next\\s+month)\\b", from: &working)
        let recurrenceEnd = extractGroup("\\b(till\\s+[^,.;]+)$", from: &working)
        let recurrenceWindow = extractGroup("\\b(for\\s+\\d+\\s+months?)\\b", from: &working)
        let recurrence = extractRecurrence(from: &working)
        let contextHint = extractContext(from: &working, recurrence: recurrence, original: original)
        let remindAt = extractRemindAt(from: &working, recurrence: recurrence)
        working = TextUtilities.replacingMatches("^to\\s+", in: working)
        let title = TextUtilities.cleanTitle(working, fallback: "reminder")

        return CapturePreviewReminderDraft(
            title: title,
            remindAtText: remindAt,
            recurrence: recurrence,
            recurrenceStartText: recurrenceStart,
            recurrenceEndText: recurrenceEnd,
            recurrenceExceptions: exceptions,
            recurrenceWindowText: recurrenceWindow,
            contextHint: contextHint
        )
    }

    private static func extractGroup(_ pattern: String, from working: inout String) -> String? {
        guard let match = TextUtilities.firstMatch(pattern, in: working),
              let value = TextUtilities.group(1, from: match, in: working) else {
            return nil
        }
        TextUtilities.removeMatch(match, from: &working)
        return TextUtilities.shadow(value)
    }

    private static func extractRecurrence(from working: inout String) -> ReminderRecurrence? {
        if let match = TextUtilities.firstMatch("\\bevery\\s+other\\s+day\\b", in: working) {
            TextUtilities.removeMatch(match, from: &working)
            return ReminderRecurrence(frequency: "daily", interval: 2)
        }
        for day in TextUtilities.weekdays {
            if let match = TextUtilities.firstMatch("\\bevery\\s+other\\s+\(day)\\b", in: working) {
                TextUtilities.removeMatch(match, from: &working)
                return ReminderRecurrence(frequency: "weekly", interval: 2, days: [day])
            }
            if let match = TextUtilities.firstMatch("\\bevery\\s+\(day)\\b", in: working) {
                TextUtilities.removeMatch(match, from: &working)
                return ReminderRecurrence(frequency: "weekly", interval: 1, days: [day])
            }
        }
        return nil
    }

    private static func extractExceptions(from working: inout String) -> [CapturePreviewRecurrenceException] {
        var exceptions: [CapturePreviewRecurrenceException] = []
        if let match = TextUtilities.firstMatch("\\b(unless\\s+.+?)(?=\\s+starting\\b|\\s+but\\b|$)", in: working),
           let raw = TextUtilities.group(1, from: match, in: working) {
            exceptions.append(CapturePreviewRecurrenceException(raw: TextUtilities.shadow(raw)))
            TextUtilities.removeMatch(match, from: &working)
        }
        if let match = TextUtilities.firstMatch("\\b(but\\s+skip\\s+[^,.;]+)$", in: working),
           let raw = TextUtilities.group(1, from: match, in: working) {
            exceptions.append(CapturePreviewRecurrenceException(raw: TextUtilities.shadow(raw)))
            TextUtilities.removeMatch(match, from: &working)
        }
        if let match = TextUtilities.firstMatch("\\b(except\\s+the\\s+week\\s+of\\s+the\\s+\\d+(?:st|nd|rd|th)?)\\b", in: working),
           let raw = TextUtilities.group(1, from: match, in: working) {
            exceptions.append(CapturePreviewRecurrenceException(raw: TextUtilities.shadow(raw)))
            TextUtilities.removeMatch(match, from: &working)
        }
        return exceptions
    }

    private static func extractContext(from working: inout String, recurrence: ReminderRecurrence?, original: String) -> String? {
        let nightIsTemporal = TextUtilities.firstMatch("\\bevery\\s+(?:other\\s+)?[a-z]+\\s+(?:at\\s+)?night\\b", in: original) != nil
            || TextUtilities.firstMatch("\\bat\\s+night\\b", in: original) != nil
        if recurrence != nil, nightIsTemporal, let match = TextUtilities.firstMatch("\\b(?:at\\s+)?night\\b", in: working) {
            TextUtilities.removeMatch(match, from: &working)
            return "night"
        }
        if let match = TextUtilities.firstMatch("\\bend\\s+of\\s+month\\b", in: working) {
            _ = match
            return "end of month"
        }
        return nil
    }

    private static func extractRemindAt(from working: inout String, recurrence: ReminderRecurrence?) -> String? {
        if recurrence != nil {
            if let match = TextUtilities.firstMatch("\\bat\\s+(\\d{1,2}\\s*(?:am|pm))\\b", in: working),
               let value = TextUtilities.group(1, from: match, in: working) {
                TextUtilities.removeMatch(match, from: &working)
                return value.replacingOccurrences(of: " ", with: "")
            }
            return nil
        }
        for pattern in [
            "\\b(tomorrow)\\b",
            "\\b(end\\s+of\\s+month)\\b",
            "\\bat\\s+(\\d{1,2}\\s*(?:am|pm))\\b",
        ] {
            if let value = extractGroup(pattern, from: &working) {
                return value.contains("am") || value.contains("pm")
                    ? value.replacingOccurrences(of: " ", with: "")
                    : value
            }
        }
        return nil
    }
}

enum CalendarLightweightParser {
    static func parse(_ input: String, request _: CapturePreviewRequest) -> CapturePreviewCalendarEventDraft {
        var working = TextUtilities.shadow(input)
        let exceptions = extractExceptions(from: &working)
        let recurrenceStart = extractGroup("\\b(starting\\s+next\\s+month)\\b", from: &working)
        let recurrenceEnd = extractGroup("\\b(till\\s+[^,.;]+)$", from: &working)
        let recurrence = extractRecurrence(from: &working)
        let location = extractLocation(from: &working)
        let (time, endTime) = extractTimeRange(from: &working)
        let date = extractGroup("\\b(tomorrow)\\b", from: &working)
        let attendees = extractAttendees(from: working)
        let title = TextUtilities.cleanTitle(working, fallback: "event")
        let duration = time == nil ? nil : 60

        return CapturePreviewCalendarEventDraft(
            title: title,
            dateText: date,
            timeText: time,
            endTimeText: endTime,
            durationMinutes: duration,
            location: location,
            recurrenceText: recurrence,
            recurrenceStartText: recurrenceStart,
            recurrenceEndText: recurrenceEnd,
            recurrenceExceptions: exceptions,
            attendees: attendees
        )
    }

    private static func extractGroup(_ pattern: String, from working: inout String) -> String? {
        guard let match = TextUtilities.firstMatch(pattern, in: working),
              let value = TextUtilities.group(1, from: match, in: working) else {
            return nil
        }
        TextUtilities.removeMatch(match, from: &working)
        return TextUtilities.shadow(value)
    }

    private static func extractRecurrence(from working: inout String) -> String? {
        for day in TextUtilities.weekdays {
            if let match = TextUtilities.firstMatch("\\bevery\\s+other\\s+\(day)\\b", in: working),
               let raw = TextUtilities.group(0, from: match, in: working) {
                TextUtilities.removeMatch(match, from: &working)
                return TextUtilities.shadow(raw)
            }
            if let match = TextUtilities.firstMatch("\\bevery\\s+\(day)\\b", in: working),
               let raw = TextUtilities.group(0, from: match, in: working) {
                TextUtilities.removeMatch(match, from: &working)
                return TextUtilities.shadow(raw)
            }
        }
        return nil
    }

    private static func extractExceptions(from working: inout String) -> [CapturePreviewRecurrenceException] {
        var exceptions: [CapturePreviewRecurrenceException] = []
        if let match = TextUtilities.firstMatch("\\b(unless\\s+.+?)(?=\\s+starting\\b|\\s+but\\b|$)", in: working),
           let raw = TextUtilities.group(1, from: match, in: working) {
            exceptions.append(CapturePreviewRecurrenceException(raw: TextUtilities.shadow(raw)))
            TextUtilities.removeMatch(match, from: &working)
        }
        if let match = TextUtilities.firstMatch("\\b(but\\s+skip\\s+[^,.;]+)$", in: working),
           let raw = TextUtilities.group(1, from: match, in: working) {
            exceptions.append(CapturePreviewRecurrenceException(raw: TextUtilities.shadow(raw)))
            TextUtilities.removeMatch(match, from: &working)
        }
        return exceptions
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

    private static func extractTimeRange(from working: inout String) -> (String?, String?) {
        if let match = TextUtilities.firstMatch("\\b(\\d{1,2}\\s*(?:am|pm))\\s+to\\s+(\\d{1,2}\\s*(?:am|pm))\\b", in: working),
           let start = TextUtilities.group(1, from: match, in: working),
           let end = TextUtilities.group(2, from: match, in: working) {
            TextUtilities.removeMatch(match, from: &working)
            return (normalizeTime(start), normalizeTime(end))
        }
        if let match = TextUtilities.firstMatch("\\bat\\s+(\\d{1,2}\\s*(?:am|pm))\\b", in: working),
           let start = TextUtilities.group(1, from: match, in: working) {
            TextUtilities.removeMatch(match, from: &working)
            return (normalizeTime(start), nil)
        }
        return (nil, nil)
    }

    private static func normalizeTime(_ value: String) -> String {
        value.replacingOccurrences(of: " ", with: "")
    }

    private static func extractAttendees(from working: String) -> [String] {
        guard let match = TextUtilities.firstMatch("\\bwith\\s+([a-z][a-z0-9-]*)\\b", in: working),
              let value = TextUtilities.group(1, from: match, in: working) else {
            return []
        }
        return [TextUtilities.shadow(value)]
    }
}

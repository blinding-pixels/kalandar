import Foundation

public struct CapturePreviewRequest: Codable, Sendable {
    public var text: String
    public var nowIso: String?
    public var timezone: String
    public var locale: String?
    public var knownAssignees: [String]?

    public init(
        text: String,
        nowIso: String? = nil,
        timezone: String = "America/New_York",
        locale: String? = "en-US",
        knownAssignees: [String]? = nil
    ) {
        self.text = text
        self.nowIso = nowIso
        self.timezone = timezone
        self.locale = locale
        self.knownAssignees = knownAssignees
    }

    enum CodingKeys: String, CodingKey {
        case text
        case nowIso = "now_iso"
        case timezone
        case locale
        case knownAssignees = "known_assignees"
    }
}

public struct CapturePreviewCandidate: Codable, Equatable, Sendable {
    public var kind: String
    public var score: Double

    public init(kind: String, score: Double) {
        self.kind = kind
        self.score = score
    }
}

public struct CapturePreviewWarning: Codable, Equatable, Sendable {
    public var code: String
    public var severity: String
    public var message: String

    public init(code: String, severity: String, message: String) {
        self.code = code
        self.severity = severity
        self.message = message
    }
}

public struct CapturePreviewRecurrenceException: Codable, Equatable, Sendable {
    public var raw: String
    public var condition: String?
    public var action: String?

    public init(raw: String, condition: String? = nil, action: String? = nil) {
        self.raw = raw
        self.condition = condition
        self.action = action
    }
}

public struct ReminderRecurrence: Codable, Equatable, Sendable {
    public var frequency: String
    public var interval: Int
    public var days: [String]?

    public init(frequency: String, interval: Int, days: [String]? = nil) {
        self.frequency = frequency
        self.interval = interval
        self.days = days
    }
}

public struct CapturePreviewTaskDraft: Codable, Equatable, Sendable {
    public var title: String
    public var assignees: [String]
    public var projectHint: String?
    public var tags: [String]
    public var priority: String?
    public var status: String?
    public var dueDateText: String?
    public var dueTimeText: String?
    public var durationText: String?
    public var recurrenceText: String?
    public var recurrenceStartText: String?
    public var recurrenceEndText: String?
    public var recurrenceExceptions: [CapturePreviewRecurrenceException]

    public init(
        title: String,
        assignees: [String] = [],
        projectHint: String? = nil,
        tags: [String] = [],
        priority: String? = nil,
        status: String? = nil,
        dueDateText: String? = nil,
        dueTimeText: String? = nil,
        durationText: String? = nil,
        recurrenceText: String? = nil,
        recurrenceStartText: String? = nil,
        recurrenceEndText: String? = nil,
        recurrenceExceptions: [CapturePreviewRecurrenceException] = []
    ) {
        self.title = title
        self.assignees = assignees
        self.projectHint = projectHint
        self.tags = tags
        self.priority = priority
        self.status = status
        self.dueDateText = dueDateText
        self.dueTimeText = dueTimeText
        self.durationText = durationText
        self.recurrenceText = recurrenceText
        self.recurrenceStartText = recurrenceStartText
        self.recurrenceEndText = recurrenceEndText
        self.recurrenceExceptions = recurrenceExceptions
    }

    enum CodingKeys: String, CodingKey {
        case title
        case assignees
        case projectHint = "project_hint"
        case tags
        case priority
        case status
        case dueDateText = "due_date_text"
        case dueTimeText = "due_time_text"
        case durationText = "duration_text"
        case recurrenceText = "recurrence_text"
        case recurrenceStartText = "recurrence_start_text"
        case recurrenceEndText = "recurrence_end_text"
        case recurrenceExceptions = "recurrence_exceptions"
    }
}

public struct CapturePreviewReminderDraft: Codable, Equatable, Sendable {
    public var title: String
    public var remindAtText: String?
    public var recurrence: ReminderRecurrence?
    public var recurrenceStartText: String?
    public var recurrenceEndText: String?
    public var recurrenceExceptions: [CapturePreviewRecurrenceException]
    public var recurrenceWindowText: String?
    public var location: String?
    public var contextHint: String?

    public init(
        title: String,
        remindAtText: String? = nil,
        recurrence: ReminderRecurrence? = nil,
        recurrenceStartText: String? = nil,
        recurrenceEndText: String? = nil,
        recurrenceExceptions: [CapturePreviewRecurrenceException] = [],
        recurrenceWindowText: String? = nil,
        location: String? = nil,
        contextHint: String? = nil
    ) {
        self.title = title
        self.remindAtText = remindAtText
        self.recurrence = recurrence
        self.recurrenceStartText = recurrenceStartText
        self.recurrenceEndText = recurrenceEndText
        self.recurrenceExceptions = recurrenceExceptions
        self.recurrenceWindowText = recurrenceWindowText
        self.location = location
        self.contextHint = contextHint
    }

    enum CodingKeys: String, CodingKey {
        case title
        case remindAtText = "remind_at_text"
        case recurrence
        case recurrenceStartText = "recurrence_start_text"
        case recurrenceEndText = "recurrence_end_text"
        case recurrenceExceptions = "recurrence_exceptions"
        case recurrenceWindowText = "recurrence_window_text"
        case location
        case contextHint = "context_hint"
    }
}

public struct CapturePreviewCalendarEventDraft: Codable, Equatable, Sendable {
    public var title: String?
    public var dateText: String?
    public var timeText: String?
    public var endTimeText: String?
    public var durationMinutes: Int?
    public var location: String?
    public var recurrenceText: String?
    public var recurrenceStartText: String?
    public var recurrenceEndText: String?
    public var recurrenceExceptions: [CapturePreviewRecurrenceException]
    public var alertOffsetMinutes: [Int]
    public var attendees: [String]

    public init(
        title: String?,
        dateText: String? = nil,
        timeText: String? = nil,
        endTimeText: String? = nil,
        durationMinutes: Int? = nil,
        location: String? = nil,
        recurrenceText: String? = nil,
        recurrenceStartText: String? = nil,
        recurrenceEndText: String? = nil,
        recurrenceExceptions: [CapturePreviewRecurrenceException] = [],
        alertOffsetMinutes: [Int] = [],
        attendees: [String] = []
    ) {
        self.title = title
        self.dateText = dateText
        self.timeText = timeText
        self.endTimeText = endTimeText
        self.durationMinutes = durationMinutes
        self.location = location
        self.recurrenceText = recurrenceText
        self.recurrenceStartText = recurrenceStartText
        self.recurrenceEndText = recurrenceEndText
        self.recurrenceExceptions = recurrenceExceptions
        self.alertOffsetMinutes = alertOffsetMinutes
        self.attendees = attendees
    }

    enum CodingKeys: String, CodingKey {
        case title
        case dateText = "date_text"
        case timeText = "time_text"
        case endTimeText = "end_time_text"
        case durationMinutes = "duration_minutes"
        case location
        case recurrenceText = "recurrence_text"
        case recurrenceStartText = "recurrence_start_text"
        case recurrenceEndText = "recurrence_end_text"
        case recurrenceExceptions = "recurrence_exceptions"
        case alertOffsetMinutes = "alert_offset_minutes"
        case attendees
    }
}

public struct CapturePreviewThoughtDraft: Codable, Equatable, Sendable {
    public var text: String

    public init(text: String) {
        self.text = text
    }
}

public struct CapturePreviewResult: Codable, Equatable, Sendable {
    public var kind: String
    public var confidence: Double
    public var candidates: [CapturePreviewCandidate]
    public var task: CapturePreviewTaskDraft?
    public var event: CapturePreviewCalendarEventDraft?
    public var reminder: CapturePreviewReminderDraft?
    public var thought: CapturePreviewThoughtDraft?
    public var unavailableReason: String?
    public var warnings: [CapturePreviewWarning]

    public init(
        kind: String,
        confidence: Double,
        candidates: [CapturePreviewCandidate] = [],
        task: CapturePreviewTaskDraft? = nil,
        event: CapturePreviewCalendarEventDraft? = nil,
        reminder: CapturePreviewReminderDraft? = nil,
        thought: CapturePreviewThoughtDraft? = nil,
        unavailableReason: String? = nil,
        warnings: [CapturePreviewWarning] = []
    ) {
        self.kind = kind
        self.confidence = confidence
        self.candidates = candidates
        self.task = task
        self.event = event
        self.reminder = reminder
        self.thought = thought
        self.unavailableReason = unavailableReason
        self.warnings = warnings
    }

    enum CodingKeys: String, CodingKey {
        case kind
        case confidence
        case candidates
        case task
        case event
        case reminder
        case thought
        case unavailableReason = "unavailable_reason"
        case warnings
    }
}

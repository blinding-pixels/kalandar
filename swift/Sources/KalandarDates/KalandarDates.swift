import Foundation

public struct ScheduleSpan: Codable, Equatable, Sendable {
    public var role: String
    public var start: Int
    public var end: Int
    public var text: String
}

public struct ScheduleTexts: Codable, Equatable, Sendable {
    public var moment: String?
    public var date: String?
    public var time: String?
    public var endTime: String?
    public var duration: String?
    public var lead: String?
    public var alert: String?
    public var recurrence: String?
    public var recurrenceStart: String?
    public var recurrenceEnd: String?
    public var recurrenceWindow: String?
    public var recurrenceExceptions: [String]

    public static let empty = ScheduleTexts(
        moment: nil, date: nil, time: nil, endTime: nil, duration: nil, lead: nil, alert: nil,
        recurrence: nil, recurrenceStart: nil, recurrenceEnd: nil, recurrenceWindow: nil, recurrenceExceptions: []
    )

    enum CodingKeys: String, CodingKey {
        case moment
        case date
        case time
        case endTime = "end_time"
        case duration
        case lead
        case alert
        case recurrence
        case recurrenceStart = "recurrence_start"
        case recurrenceEnd = "recurrence_end"
        case recurrenceWindow = "recurrence_window"
        case recurrenceExceptions = "recurrence_exceptions"
    }
}

public struct PreviewMoment: Codable, Equatable, Sendable {
    public var date: String
    public var time: String?
    public var local: String?
    public var instant: String?
    public var allDay: Bool
    public var implied: [String]
    public var alternativeDate: String?

    enum CodingKeys: String, CodingKey {
        case date
        case time
        case local
        case instant
        case allDay = "all_day"
        case implied
        case alternativeDate = "alternative_date"
    }
}

public struct PreviewWeekday: Codable, Equatable, Sendable {
    public var position: Int?
    public var weekday: String
}

public struct PreviewRecurrenceException: Codable, Equatable, Sendable {
    public var raw: String
    public var condition: String?
    public var action: String?
}

public struct PreviewRecurrence: Codable, Equatable, Sendable {
    public var rrule: String
    public var frequency: String
    public var interval: Int
    public var days: [String]?
    public var weekdays: [PreviewWeekday]
    public var monthDays: [Int]
    public var months: [Int]
    public var time: String?
    public var count: Int?
    public var startDate: String?
    public var endDate: String?
    public var until: String?
    public var exdates: [String]
    public var exceptions: [PreviewRecurrenceException]

    enum CodingKeys: String, CodingKey {
        case rrule
        case frequency
        case interval
        case days
        case weekdays
        case monthDays = "month_days"
        case months
        case time
        case count
        case startDate = "start_date"
        case endDate = "end_date"
        case until
        case exdates
        case exceptions
    }
}

public struct PreviewSchedule: Codable, Equatable, Sendable {
    public var start: PreviewMoment?
    public var end: PreviewMoment?
    public var durationMinutes: Int?
    public var recurrence: PreviewRecurrence?
    public var leadMinutes: Int?
    public var alertMinutesBefore: Int?
    public var alertAt: PreviewMoment?
    public var alertsOff: Bool
    public var notifyAt: String?
    public var taskSignal: String?

    enum CodingKeys: String, CodingKey {
        case start
        case end
        case durationMinutes = "duration_minutes"
        case recurrence
        case leadMinutes = "lead_minutes"
        case alertMinutesBefore = "alert_minutes_before"
        case alertAt = "alert_at"
        case alertsOff = "alerts_off"
        case notifyAt = "notify_at"
        case taskSignal = "task_signal"
    }
}

public struct PreviewWarning: Codable, Equatable, Sendable {
    public var code: String
    public var message: String
}

public struct CaptureSchedule: Codable, Equatable, Sendable {
    public var spans: [ScheduleSpan]
    public var texts: ScheduleTexts
    public var schedule: PreviewSchedule?
    public var warnings: [PreviewWarning]
}

public struct HolidayDataInfo: Codable, Equatable, Sendable {
    public var source: String
    public var version: String
    public var integrity: String
    public var generated: String
    public var firstYear: Int
    public var lastYear: Int

    enum CodingKeys: String, CodingKey {
        case source
        case version
        case integrity
        case generated
        case firstYear = "first_year"
        case lastYear = "last_year"
    }
}

public enum KalandarDatesError: Error, LocalizedError, Equatable {
    case resolver(String)

    public var errorDescription: String? {
        switch self {
        case .resolver(let message):
            return "kalandar-dates: \(message)"
        }
    }
}

public struct ScheduleLabel: Codable, Equatable, Sendable {
    public var label: String
    public var start: Int
    public var end: Int

    public init(label: String, start: Int, end: Int) {
        self.label = label
        self.start = start
        self.end = end
    }
}

public enum KalandarDates {
    private struct Request: Encodable {
        var text: String
        var kind: String
        var now: String
        var timezone: String
        var locale: String?
        var spans: [ScheduleLabel]?
        var offsets = "utf16"
    }

    private struct Failure: Decodable {
        var error: String
    }

    public static func captureSchedule(
        text: String,
        kind: String,
        nowISO: String? = nil,
        timezone: String = "UTC",
        locale: String? = nil,
        spans: [ScheduleLabel]? = nil
    ) throws -> CaptureSchedule {
        let request = Request(
            text: text,
            kind: kind,
            now: nowISO ?? ISO8601DateFormatter().string(from: Date()),
            timezone: timezone,
            locale: locale,
            spans: spans
        )
        let payload = String(decoding: try JSONEncoder().encode(request), as: UTF8.self)
        let response = Data(captureScheduleJson(request: payload).utf8)
        if let failure = try? JSONDecoder().decode(Failure.self, from: response) {
            throw KalandarDatesError.resolver(failure.error)
        }
        return try JSONDecoder().decode(CaptureSchedule.self, from: response)
    }

    public static func capturePreviewJSON(_ request: String) -> String {
        capturePreviewJson(request: request)
    }

    public static func holidayData() throws -> HolidayDataInfo {
        try JSONDecoder().decode(HolidayDataInfo.self, from: Data(holidayDataJson().utf8))
    }

    public static func removeSpans(_ spans: [ScheduleSpan], from text: String) -> String {
        let source = text as NSString
        var output = ""
        var cursor = 0
        for span in spans.sorted(by: { ($0.start, $0.end) < ($1.start, $1.end) }) {
            if span.start >= cursor {
                output += source.substring(with: NSRange(location: cursor, length: span.start - cursor)) + " "
            }
            cursor = max(cursor, span.end)
        }
        if cursor < source.length {
            output += source.substring(from: cursor)
        }
        return output.split(whereSeparator: \.isWhitespace).joined(separator: " ")
    }
}

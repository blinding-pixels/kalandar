import Foundation
import KalandarDates
import Testing
@testable import KalandarCoreAIRuntime

@Suite("kalandar-dates through the Rust binding")
struct KalandarDatesTests {
    @Test("resolves a task due date and time")
    func resolvesTaskDue() throws {
        let found = try KalandarDates.captureSchedule(
            text: "finish report tomorrow at 3pm",
            kind: "task",
            nowISO: "2026-05-16T12:00:00Z",
            timezone: "America/New_York"
        )
        #expect(found.texts.date == "tomorrow")
        #expect(found.texts.time == "3pm")
        #expect(found.schedule?.start?.local == "2026-05-17T15:00:00")
        #expect(found.spans.first?.role == "due_date")
    }

    @Test("resolves a repeat with its time")
    func resolvesRepeat() throws {
        let found = try KalandarDates.captureSchedule(
            text: "water plants every other saturday at 9am",
            kind: "reminder",
            nowISO: "2026-05-16T12:00:00Z",
            timezone: "America/New_York"
        )
        #expect(found.schedule?.recurrence?.rrule == "FREQ=WEEKLY;INTERVAL=2;BYDAY=SA")
        #expect(found.schedule?.recurrence?.time == "09:00")
    }

    @Test("exposes the holiday data version")
    func holidayVersion() throws {
        let info = try KalandarDates.holidayData()
        #expect(info.source == "date-holidays")
        #expect(info.integrity.hasPrefix("sha512-"))
    }

    @Test("puts the resolved schedule on the capture preview")
    func previewCarriesSchedule() throws {
        let result = LightweightCaptureParser().parse(CapturePreviewRequest(
            text: "task: finish report tomorrow at 3pm",
            nowIso: "2026-05-16T12:00:00.000Z",
            timezone: "America/New_York"
        ))
        #expect(result.task?.dueDateText == "tomorrow")
        #expect(result.task?.dueTimeText == "3pm")
        #expect(result.schedule?.start?.instant == "2026-05-17T15:00:00-04:00")
    }
}

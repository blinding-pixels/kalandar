import Testing
@testable import KalandarCoreAIRuntime

@Suite("Swift capture preview runtime")
struct LightweightCaptureParserTests {
    @Test("parses task metadata")
    func parsesTaskMetadata() {
        let parser = LightweightCaptureParser()
        let result = parser.parse(CapturePreviewRequest(
            text: "task: finish report tomorrow at 3pm with Sara #finance high priority",
            knownAssignees: ["Sara"]
        ))
        #expect(result.kind == "task")
        #expect(result.task?.title == "finish report")
        #expect(result.task?.assignees == ["sara"])
        #expect(result.task?.tags == ["finance"])
        #expect(result.task?.priority == "high")
    }

    @Test("parses recurring reminder exception")
    func parsesReminderException() {
        let parser = LightweightCaptureParser()
        let result = parser.parse(CapturePreviewRequest(
            text: "reminder: Water Plants every other saturday except the week of the 21st and add soil"
        ))
        #expect(result.kind == "reminder")
        #expect(result.reminder?.title == "water plants and add soil")
        #expect(result.reminder?.recurrence?.interval == 2)
        #expect(result.reminder?.recurrenceExceptions.first?.raw == "except the week of the 21st")
    }
}

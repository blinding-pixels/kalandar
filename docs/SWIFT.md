# Swift And Core AI

`KalandarCoreAIRuntime` is a Swift Package Manager product distributed from this Git repository. Add it directly to an Apple-native app, then provide a separate Core AI artifact root containing the `task`, `reminder`, and `calendar` models.

Use `CoreAIParserSuite` when the caller already knows the parser family. It returns a typed Swift draft, decoded entity spans, and model metadata. Use `CoreAICapturePreviewService` for mixed capture text and the shared `capture-preview-v1` result.

The package builds on SDKs without Core AI. Actual `.aimodel` execution requires an Apple SDK that includes the `CoreAI` framework, a valid artifact root, and macOS or iOS 27 or later at runtime.

Dates, times, repeats, lead times and holidays come from the Rust resolver `kalandar-dates`, which the `KalandarDates` product calls through a UniFFI binding to the `KalandarDatesFFI` XCFramework. In a source checkout, build the XCFramework once with `npm run dates:xcframework`. Tagged releases download it as a release asset. You can also call it directly:

```swift
import KalandarDates

let found = try KalandarDates.captureSchedule(
    text: "water plants every other saturday at 9am",
    kind: "reminder",
    timezone: "America/New_York"
)
print(found.schedule?.recurrence?.rrule ?? "")
```

For setup and examples, see [Use Swift and Core AI](./tasks/use-coreai.md) and [Core AI runtime](../coreai/README.md).

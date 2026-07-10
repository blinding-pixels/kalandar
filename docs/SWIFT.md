# Swift And Core AI

`KalandarCoreAIRuntime` is a Swift Package Manager product distributed from this Git repository. Add it directly to an Apple-native app, then provide a separate Core AI artifact root containing the `task`, `reminder`, and `calendar` models.

Use `CoreAIParserSuite` when the caller already knows the parser family. It returns a typed Swift draft, decoded entity spans, and model metadata. Use `CoreAICapturePreviewService` for mixed capture text and the shared `capture-preview-v1` result.

The package builds on SDKs without Core AI. Actual `.aimodel` execution requires an Apple SDK that includes the `CoreAI` framework, a valid artifact root, and macOS or iOS 27 or later at runtime.

For setup and examples, see [Use Swift and Core AI](./tasks/use-coreai.md) and [Core AI runtime](../coreai/README.md).

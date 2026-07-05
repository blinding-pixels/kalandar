import Foundation
import KalandarCoreAIRuntime

struct CLI {
    var artifactRoot: URL?
    var requireCoreAISmoke = false

    init(arguments: [String], environment: [String: String]) {
        var index = 1
        var root = environment["KALANDAR_COREAI_ARTIFACT_ROOT"]
        var requireSmoke = environment["KALANDAR_COREAI_REQUIRE_SMOKE"] == "1"

        while index < arguments.count {
            switch arguments[index] {
            case "--artifact-root":
                if index + 1 < arguments.count {
                    root = arguments[index + 1]
                    index += 1
                }
            case "--require-coreai-smoke":
                requireSmoke = true
            default:
                break
            }
            index += 1
        }

        if let root, !root.isEmpty {
            artifactRoot = URL(fileURLWithPath: NSString(string: root).expandingTildeInPath)
        }
        requireCoreAISmoke = requireSmoke
    }
}

@main
enum Main {
    static func main() async {
        let cli = CLI(arguments: CommandLine.arguments, environment: ProcessInfo.processInfo.environment)
        do {
            let service = try CoreAICapturePreviewService(
                artifactRoot: cli.artifactRoot,
                requireCoreAISmoke: cli.requireCoreAISmoke
            )
            let input = FileHandle.standardInput.readDataToEndOfFile()
            guard let line = String(data: input, encoding: .utf8)?.split(separator: "\n").first else {
                throw KalandarCoreAIError.invalidManifest("stdin must contain one capture request JSON line")
            }
            let request = try JSONDecoder().decode(CapturePreviewRequest.self, from: Data(line.utf8))
            let result = try await service.preview(request)
            let encoder = JSONEncoder()
            encoder.outputFormatting = [.sortedKeys]
            let payload = try encoder.encode(result)
            FileHandle.standardOutput.write(payload)
            FileHandle.standardOutput.write(Data("\n".utf8))
        } catch {
            let message = (error as? LocalizedError)?.errorDescription ?? String(describing: error)
            FileHandle.standardError.write(Data("\(message)\n".utf8))
            Foundation.exit(1)
        }
    }
}

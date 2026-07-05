import Foundation

public enum KalandarCoreAIError: Error, LocalizedError, Sendable {
    case artifactRootMissing(String)
    case artifactMissing(String)
    case invalidManifest(String)
    case coreAIUnavailable
    case modelOutputMissing(String)

    public var errorDescription: String? {
        switch self {
        case .artifactRootMissing(let path):
            return "Core AI artifact root does not exist: \(path)"
        case .artifactMissing(let path):
            return "Core AI artifact is missing: \(path)"
        case .invalidManifest(let message):
            return "Invalid Core AI artifact manifest: \(message)"
        case .coreAIUnavailable:
            return "Core AI runtime is unavailable. Build with Xcode 27 / macOS or iOS 27 SDK."
        case .modelOutputMissing(let name):
            return "Core AI model output is missing: \(name)"
        }
    }
}

public struct CoreAIArtifactManifest: Decodable, Sendable {
    public struct TensorSpec: Decodable, Sendable {
        public var name: String
        public var dtype: String
        public var shape: [Int]
    }

    public struct InputContract: Decodable, Sendable {
        public var inputs: [TensorSpec]
        public var outputs: [TensorSpec]
        public var maxSequenceLength: Int

        enum CodingKeys: String, CodingKey {
            case inputs
            case outputs
            case maxSequenceLength = "max_sequence_length"
        }
    }

    public struct TokenizerSpec: Decodable, Sendable {
        public var kind: String
        public var path: String
    }

    public var manifestVersion: String
    public var runtimeKind: String
    public var modelKind: String
    public var modelAsset: String
    public var inputContract: InputContract
    public var labels: [String: String]?
    public var tokenizer: TokenizerSpec

    enum CodingKeys: String, CodingKey {
        case manifestVersion = "manifest_version"
        case runtimeKind = "runtime_kind"
        case modelKind = "model_kind"
        case modelAsset = "model_asset"
        case inputContract = "input_contract"
        case labels
        case tokenizer
    }
}

public struct CoreAIParserArtifact: Sendable {
    public var family: String
    public var root: URL
    public var manifest: CoreAIArtifactManifest
    public var modelURL: URL
    public var tokenizerURL: URL

    public static func load(family: String, from artifactRoot: URL) throws -> Self {
        let familyRoot = artifactRoot.appendingPathComponent(family, isDirectory: true)
        let manifestURL = familyRoot.appendingPathComponent("manifest.json")
        guard FileManager.default.fileExists(atPath: manifestURL.path) else {
            throw KalandarCoreAIError.artifactMissing(manifestURL.path)
        }

        let data = try Data(contentsOf: manifestURL)
        let manifest = try JSONDecoder().decode(CoreAIArtifactManifest.self, from: data)
        guard manifest.manifestVersion == "kalandar-coreai-model-v1" else {
            throw KalandarCoreAIError.invalidManifest("unsupported manifest_version \(manifest.manifestVersion)")
        }
        guard manifest.runtimeKind == "coreai" else {
            throw KalandarCoreAIError.invalidManifest("unsupported runtime_kind \(manifest.runtimeKind)")
        }
        guard manifest.tokenizer.kind == "bert_wordpiece" else {
            throw KalandarCoreAIError.invalidManifest("unsupported tokenizer kind \(manifest.tokenizer.kind)")
        }

        let modelURL = familyRoot.appendingPathComponent(manifest.modelAsset, isDirectory: true)
        let tokenizerURL = familyRoot.appendingPathComponent(manifest.tokenizer.path, isDirectory: true)
        guard FileManager.default.fileExists(atPath: modelURL.path) else {
            throw KalandarCoreAIError.artifactMissing(modelURL.path)
        }
        guard FileManager.default.fileExists(atPath: tokenizerURL.path) else {
            throw KalandarCoreAIError.artifactMissing(tokenizerURL.path)
        }

        return CoreAIParserArtifact(
            family: family,
            root: familyRoot,
            manifest: manifest,
            modelURL: modelURL,
            tokenizerURL: tokenizerURL
        )
    }
}

public struct CoreAIArtifactStore: Sendable {
    public var root: URL

    public init(root: URL) throws {
        var isDirectory: ObjCBool = false
        guard FileManager.default.fileExists(atPath: root.path, isDirectory: &isDirectory), isDirectory.boolValue else {
            throw KalandarCoreAIError.artifactRootMissing(root.path)
        }
        self.root = root
    }

    public func artifact(family: String) throws -> CoreAIParserArtifact {
        try CoreAIParserArtifact.load(family: family, from: root)
    }
}

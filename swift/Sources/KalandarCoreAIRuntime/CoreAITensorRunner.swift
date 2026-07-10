import Foundation

public struct CoreAITensorOutput: Sendable, Equatable {
    public var name: String
    public var shape: [Int]
    public var values: [Float]
}

public protocol CoreAITensorRunning: Sendable {
    func runTokenClassifier(artifact: CoreAIParserArtifact, tokenized: TokenizedText) async throws -> CoreAITensorOutput
}

#if canImport(CoreAI)
import CoreAI

public struct CoreAITensorRunner: CoreAITensorRunning {
    public init() {}

    public func runTokenClassifier(artifact: CoreAIParserArtifact, tokenized: TokenizedText) async throws -> CoreAITensorOutput {
        guard #available(macOS 27.0, iOS 27.0, *) else {
            throw KalandarCoreAIError.coreAIUnavailable
        }
        return try await runAvailableTokenClassifier(artifact: artifact, tokenized: tokenized)
    }

    @available(macOS 27.0, iOS 27.0, *)
    private func runAvailableTokenClassifier(
        artifact: CoreAIParserArtifact,
        tokenized: TokenizedText
    ) async throws -> CoreAITensorOutput {
        let model = try await AIModel(contentsOf: artifact.modelURL)
        guard let function = try model.loadFunction(named: "main") else {
            throw KalandarCoreAIError.invalidManifest("model does not expose main function")
        }

        var inputs: [String: NDArray] = [:]
        inputs["input_ids"] = try makeInt32Array(function: function, name: "input_ids", values: tokenized.inputIds)
        inputs["attention_mask"] = try makeInt32Array(function: function, name: "attention_mask", values: tokenized.attentionMask)

        var outputs = try await function.run(inputs: inputs)
        let outputName = artifact.manifest.inputContract.outputs.first?.name ?? "logits"
        guard let array = outputs.remove(outputName)?.ndArray else {
            throw KalandarCoreAIError.modelOutputMissing(outputName)
        }

        return CoreAITensorOutput(name: outputName, shape: array.shape, values: readFloats(array))
    }

    @available(macOS 27.0, iOS 27.0, *)
    private func makeInt32Array(function: InferenceFunction, name: String, values: [Int32]) throws -> NDArray {
        guard case .ndArray(let descriptor) = function.descriptor.inputDescriptor(of: name) else {
            throw KalandarCoreAIError.invalidManifest("model input is not an NDArray: \(name)")
        }
        let resolved = descriptor.resolvingDynamicDimensions([1, values.count])
        var array = NDArray(descriptor: resolved)
        var view = array.mutableView(as: Int32.self)
        view.withUnsafeMutablePointer { pointer, _, _ in
            for index in 0..<values.count {
                pointer[index] = values[index]
            }
        }
        return array
    }

    @available(macOS 27.0, iOS 27.0, *)
    private func readFloats(_ array: NDArray) -> [Float] {
        let total = array.shape.reduce(1, *)
        var result = Array(repeating: Float(0), count: total)
        switch array.scalarType {
        case .float32:
            array.view(as: Float.self).withUnsafePointer { pointer, _, _ in
                for index in 0..<total { result[index] = pointer[index] }
            }
        #if !((os(macOS) || targetEnvironment(macCatalyst)) && arch(x86_64))
        case .float16:
            array.view(as: Float16.self).withUnsafePointer { pointer, _, _ in
                for index in 0..<total { result[index] = Float(pointer[index]) }
            }
        #endif
        default:
            preconditionFailure("unsupported Core AI output scalar type \(array.scalarType)")
        }
        return result
    }
}
#else
public struct CoreAITensorRunner: CoreAITensorRunning {
    public init() {}

    public func runTokenClassifier(artifact _: CoreAIParserArtifact, tokenized _: TokenizedText) async throws -> CoreAITensorOutput {
        throw KalandarCoreAIError.coreAIUnavailable
    }
}
#endif

public struct CoreAISmokeVerifier: Sendable {
    private let runner: any CoreAITensorRunning

    public init(runner: any CoreAITensorRunning = CoreAITensorRunner()) {
        self.runner = runner
    }

    public func verify(artifact: CoreAIParserArtifact, sample: String) async throws {
        let tokenizer = try BertWordPieceTokenizer(artifact: artifact)
        let tokenized = tokenizer.encode(sample)
        _ = try await runner.runTokenClassifier(artifact: artifact, tokenized: tokenized)
    }
}

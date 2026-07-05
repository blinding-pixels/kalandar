import Foundation

enum TextUtilities {
    static let weekdays = [
        "monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday",
    ]

    static func shadow(_ input: String) -> String {
        input
            .replacingOccurrences(of: "\\s+", with: " ", options: .regularExpression)
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
    }

    static func cleanTitle(_ input: String, fallback: String) -> String {
        var output = input
            .replacingOccurrences(of: "[,;:]+", with: " ", options: .regularExpression)
            .replacingOccurrences(of: "\\s+", with: " ", options: .regularExpression)
            .trimmingCharacters(in: CharacterSet.whitespacesAndNewlines.union(CharacterSet(charactersIn: ".,;:-")))
        output = output.replacingOccurrences(of: "\\b(?:please|pls|priority)\\b", with: " ", options: .regularExpression)
        output = output.replacingOccurrences(of: "\\s+", with: " ", options: .regularExpression)
            .trimmingCharacters(in: CharacterSet.whitespacesAndNewlines.union(CharacterSet(charactersIn: ".,;:-")))
        return output.isEmpty ? fallback : output
    }

    static func stripPrefix(_ text: String, prefix: String) -> String? {
        if text.lowercased().hasPrefix(prefix.lowercased()) {
            return String(text.dropFirst(prefix.count)).trimmingCharacters(in: .whitespacesAndNewlines)
        }
        return nil
    }

    static func firstMatch(_ pattern: String, in text: String) -> NSTextCheckingResult? {
        let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive])
        let range = NSRange(text.startIndex..<text.endIndex, in: text)
        return regex?.firstMatch(in: text, options: [], range: range)
    }

    static func matches(_ pattern: String, in text: String) -> [NSTextCheckingResult] {
        guard let regex = try? NSRegularExpression(pattern: pattern, options: [.caseInsensitive]) else {
            return []
        }
        let range = NSRange(text.startIndex..<text.endIndex, in: text)
        return regex.matches(in: text, options: [], range: range)
    }

    static func group(_ index: Int, from match: NSTextCheckingResult, in text: String) -> String? {
        let range = match.range(at: index)
        guard range.location != NSNotFound, let swiftRange = Range(range, in: text) else {
            return nil
        }
        return String(text[swiftRange]).trimmingCharacters(in: .whitespacesAndNewlines)
    }

    static func replacingMatches(_ pattern: String, in text: String, with replacement: String = " ") -> String {
        text.replacingOccurrences(of: pattern, with: replacement, options: [.regularExpression, .caseInsensitive])
            .replacingOccurrences(of: "\\s+", with: " ", options: .regularExpression)
            .trimmingCharacters(in: .whitespacesAndNewlines)
    }

    static func removeMatch(_ match: NSTextCheckingResult, from text: inout String) {
        guard let range = Range(match.range(at: 0), in: text) else { return }
        text.replaceSubrange(range, with: " ")
        text = shadow(text)
    }

    static func uniqueSorted(_ values: [String]) -> [String] {
        Array(Set(values.map(shadow).filter { !$0.isEmpty })).sorted()
    }
}

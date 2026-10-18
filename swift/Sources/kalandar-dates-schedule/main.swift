import Foundation
import KalandarDates

while let line = readLine(strippingNewline: true) {
    if line.trimmingCharacters(in: .whitespaces).isEmpty {
        continue
    }
    print(captureScheduleJson(request: line))
}

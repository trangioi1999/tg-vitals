import Foundation

public enum Format {
    /// 1536 → "1.5 KB". Uses base 1024 like Activity Monitor.
    public static func bytes(_ value: UInt64) -> String {
        let units = ["B", "KB", "MB", "GB", "TB"]
        var size = Double(value)
        var unit = 0
        while size >= 1024, unit < units.count - 1 {
            size /= 1024
            unit += 1
        }
        if unit == 0 { return "\(value) B" }
        let number = String(format: size < 10 ? "%.1f" : "%.0f", size)
        return "\(number) \(units[unit])"
    }

    public static func rate(_ bytesPerSecond: Double) -> String {
        bytes(UInt64(max(0, bytesPerSecond))) + "/s"
    }

    /// 0.423 → "42%".
    public static func percent(_ fraction: Double) -> String {
        "\(Int((fraction * 100).rounded()))%"
    }
}

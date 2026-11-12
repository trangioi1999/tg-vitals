import AppKit
import Charts
import SwiftUI
import VitalsCore

struct MenuBarView: View {
    @Environment(SystemMonitor.self) private var monitor
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            MetricRow(title: "CPU", value: Format.percent(monitor.cpu?.total ?? 0),
                      history: monitor.cpuHistory.elements, color: .green, maxValue: 1)

            MetricRow(title: "Bộ nhớ",
                      value: monitor.memory.map { "\(Format.bytes($0.used)) / \(Format.bytes($0.total))" } ?? "—",
                      history: monitor.memoryHistory.elements, color: .blue, maxValue: 1)

            MetricRow(title: "Mạng ↓",
                      value: Format.rate(monitor.network?.bytesInPerSecond ?? 0),
                      history: monitor.networkInHistory.elements, color: .orange, maxValue: nil)

            Divider()

            HStack {
                Button("Mở tg-vitals") {
                    openWindow(id: "main")
                    NSApp.activate(ignoringOtherApps: true)
                }
                Spacer()
                Button("Thoát") { NSApp.terminate(nil) }
            }
        }
        .padding(14)
        .frame(width: 300)
    }
}

/// Title + current value + 60-second sparkline. Reused by the overview page.
struct MetricRow: View {
    let title: String
    let value: String
    let history: [Double]
    let color: Color
    /// nil = auto-scale (network); 1 = fraction metrics.
    let maxValue: Double?

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(title).font(.headline)
                Spacer()
                Text(value).monospacedDigit().foregroundStyle(.secondary)
            }
            Chart(Array(history.enumerated()), id: \.offset) { point in
                AreaMark(x: .value("t", point.offset), y: .value(title, point.element))
                    .foregroundStyle(color.opacity(0.25))
                LineMark(x: .value("t", point.offset), y: .value(title, point.element))
                    .foregroundStyle(color)
            }
            .chartXScale(domain: 0...(SystemMonitor.historyLength - 1))
            .chartYScale(domain: 0...(maxValue ?? max(history.max() ?? 1, 1)))
            .chartXAxis(.hidden)
            .chartYAxis(.hidden)
            .frame(height: 36)
        }
    }
}

import SwiftUI
import VitalsCore

struct OverviewView: View {
    @Environment(SystemMonitor.self) private var monitor

    private let columns = [GridItem(.adaptive(minimum: 280), spacing: 16)]

    var body: some View {
        ScrollView {
            LazyVGrid(columns: columns, spacing: 16) {
                card {
                    MetricRow(title: "CPU", value: Format.percent(monitor.cpu?.total ?? 0),
                              history: monitor.cpuHistory.elements, color: .green, maxValue: 1)
                    if let cpu = monitor.cpu {
                        Text("User \(Format.percent(cpu.user)) · System \(Format.percent(cpu.system))")
                            .font(.caption).foregroundStyle(.secondary)
                        CoreBars(values: cpu.perCore)
                    }
                }

                card {
                    MetricRow(title: "Bộ nhớ", value: Format.percent(monitor.memory?.usedFraction ?? 0),
                              history: monitor.memoryHistory.elements, color: .blue, maxValue: 1)
                    if let m = monitor.memory {
                        Text("App \(Format.bytes(m.app)) · Wired \(Format.bytes(m.wired)) · Nén \(Format.bytes(m.compressed))")
                            .font(.caption).foregroundStyle(.secondary)
                    }
                }

                card {
                    MetricRow(title: "Mạng ↓", value: Format.rate(monitor.network?.bytesInPerSecond ?? 0),
                              history: monitor.networkInHistory.elements, color: .orange, maxValue: nil)
                    MetricRow(title: "Mạng ↑", value: Format.rate(monitor.network?.bytesOutPerSecond ?? 0),
                              history: monitor.networkOutHistory.elements, color: .purple, maxValue: nil)
                }
            }
            .padding(20)
        }
        .navigationTitle("Tổng quan")
    }

    private func card(@ViewBuilder _ content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 10, content: content)
            .padding(16)
            .frame(maxWidth: .infinity, alignment: .topLeading)
            .background(.background.secondary, in: RoundedRectangle(cornerRadius: 12))
    }
}

/// One small vertical bar per core.
struct CoreBars: View {
    let values: [Double]

    var body: some View {
        HStack(alignment: .bottom, spacing: 3) {
            ForEach(Array(values.enumerated()), id: \.offset) { _, value in
                RoundedRectangle(cornerRadius: 2)
                    .fill(.green.opacity(0.3 + 0.7 * value))
                    .frame(height: max(2, 30 * value))
                    .frame(maxWidth: .infinity, maxHeight: 30, alignment: .bottom)
            }
        }
        .frame(height: 30)
    }
}

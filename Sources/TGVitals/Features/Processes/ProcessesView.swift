import Darwin
import SwiftUI
import VitalsCore

struct ProcessesView: View {
    @Environment(SystemMonitor.self) private var monitor
    @State private var search = ""
    @State private var sortOrder = [KeyPathComparator(\ProcessInfoSnapshot.cpu, order: .reverse)]
    @State private var selection: Int32?
    @State private var pendingForceQuit: ProcessInfoSnapshot?

    private var rows: [ProcessInfoSnapshot] {
        let filtered = search.isEmpty
            ? monitor.processes
            : monitor.processes.filter { $0.name.localizedCaseInsensitiveContains(search) || "\($0.pid)" == search }
        return filtered.sorted(using: sortOrder)
    }

    var body: some View {
        Table(rows, selection: $selection, sortOrder: $sortOrder) {
            TableColumn("Tên", value: \.name)
            TableColumn("CPU", value: \.cpu) { Text(Format.percent($0.cpu)).monospacedDigit() }
                .width(70)
            TableColumn("Bộ nhớ", value: \.memory) { Text(Format.bytes($0.memory)).monospacedDigit() }
                .width(90)
            TableColumn("Luồng", value: \.threads) { Text("\($0.threads)").monospacedDigit() }
                .width(60)
            TableColumn("PID", value: \.pid) { Text("\($0.pid)").monospacedDigit() }
                .width(70)
        }
        .contextMenu(forSelectionType: Int32.self) { pids in
            if let pid = pids.first, let process = monitor.processes.first(where: { $0.pid == pid }) {
                Button("Thoát \(process.name)") { kill(pid, SIGTERM) }
                Button("Buộc thoát…") { pendingForceQuit = process }
            }
        }
        .searchable(text: $search, prompt: "Tìm process hoặc PID")
        .navigationTitle("Process (\(monitor.processes.count))")
        .confirmationDialog(
            "Buộc thoát \(pendingForceQuit?.name ?? "")?",
            isPresented: Binding(get: { pendingForceQuit != nil }, set: { if !$0 { pendingForceQuit = nil } })
        ) {
            Button("Buộc thoát", role: .destructive) {
                if let pid = pendingForceQuit?.pid { kill(pid, SIGKILL) }
                pendingForceQuit = nil
            }
        } message: {
            Text("Dữ liệu chưa lưu trong process này sẽ mất.")
        }
    }
}

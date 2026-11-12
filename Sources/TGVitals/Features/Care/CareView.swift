import SwiftUI
import VitalsCore

struct CareView: View {
    @Environment(CareModel.self) private var care
    @State private var confirming = false

    var body: some View {
        VStack(spacing: 0) {
            switch care.phase {
            case .idle:
                placeholder(
                    icon: "sparkles",
                    title: "Quét rác trên máy",
                    detail: "Chỉ đọc, không xóa gì. Bạn sẽ chọn trước khi dọn, và mọi thứ đều vào Thùng rác."
                )
            case .scanning:
                ProgressView("Đang quét…").frame(maxWidth: .infinity, maxHeight: .infinity)
            case .cleaning:
                ProgressView("Đang chuyển vào Thùng rác…").frame(maxWidth: .infinity, maxHeight: .infinity)
            case .done(let report):
                placeholder(
                    icon: "checkmark.circle",
                    title: "Đã dọn \(Format.bytes(report.trashedBytes))",
                    detail: summary(of: report)
                )
            case .ready:
                resultsList
            }

            Divider()
            footer
        }
        .navigationTitle("Dọn dẹp")
        .confirmationDialog(
            "Chuyển \(Format.bytes(care.selectedBytes)) vào Thùng rác?",
            isPresented: $confirming
        ) {
            Button("Chuyển vào Thùng rác", role: .destructive) { care.cleanSelected() }
        } message: {
            Text("Vẫn khôi phục được từ Thùng rác cho đến khi bạn đổ nó.")
        }
    }

    private var resultsList: some View {
        List {
            ForEach(care.results) { result in
                Section {
                    ForEach(result.items) { item in
                        Toggle(isOn: binding(for: item.url)) {
                            HStack {
                                Text(item.name).lineLimit(1).truncationMode(.middle)
                                Spacer()
                                Text(Format.bytes(item.size)).monospacedDigit().foregroundStyle(.secondary)
                            }
                        }
                    }
                } header: {
                    HStack {
                        Toggle(isOn: Binding(
                            get: { Set(result.items.map(\.url)).isSubset(of: care.selected) },
                            set: { _ in care.toggle(result) }
                        )) {
                            VStack(alignment: .leading) {
                                Text(result.target.title).font(.headline)
                                Text(result.target.note).font(.caption).foregroundStyle(.secondary)
                            }
                        }
                        Spacer()
                        Text(Format.bytes(result.totalSize)).monospacedDigit()
                    }
                }
            }
        }
    }

    private var footer: some View {
        HStack {
            if care.phase == .ready {
                Text("Đã chọn \(Format.bytes(care.selectedBytes)) / \(Format.bytes(care.totalBytes))")
                    .monospacedDigit()
            }
            Spacer()
            Button(care.phase == .ready ? "Quét lại" : "Quét") { care.scan() }
                .disabled(care.phase == .scanning || care.phase == .cleaning)
            if care.phase == .ready {
                Button("Dọn…") { confirming = true }
                    .buttonStyle(.borderedProminent)
                    .disabled(care.selected.isEmpty)
            }
        }
        .padding(12)
    }

    private func binding(for url: URL) -> Binding<Bool> {
        Binding(
            get: { care.selected.contains(url) },
            set: { isOn in
                if isOn { care.selected.insert(url) } else { care.selected.remove(url) }
            }
        )
    }

    private func placeholder(icon: String, title: String, detail: String) -> some View {
        VStack(spacing: 10) {
            Image(systemName: icon).font(.system(size: 40)).foregroundStyle(.tint)
            Text(title).font(.title2.bold())
            Text(detail).foregroundStyle(.secondary).multilineTextAlignment(.center).frame(maxWidth: 420)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func summary(of report: Trasher.Report) -> String {
        var parts = ["\(report.trashed.count) mục đã vào Thùng rác."]
        if !report.failed.isEmpty {
            parts.append("\(report.failed.count) mục không xóa được (đang dùng hoặc thiếu quyền — thử cấp Full Disk Access).")
        }
        if !report.rejected.isEmpty {
            parts.append("\(report.rejected.count) mục bị chặn vì nằm ngoài vùng an toàn.")
        }
        return parts.joined(separator: " ")
    }
}

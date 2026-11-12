import SwiftUI

enum Page: String, CaseIterable, Identifiable {
    case overview = "Tổng quan"
    case processes = "Process"
    case care = "Dọn dẹp"

    var id: Self { self }

    var icon: String {
        switch self {
        case .overview: "gauge.with.dots.needle.33percent"
        case .processes: "list.bullet.rectangle"
        case .care: "sparkles"
        }
    }
}

struct MainView: View {
    @Environment(SystemMonitor.self) private var monitor
    @State private var page: Page

    init(initialPage: Page = .overview) {
        _page = State(initialValue: initialPage)
    }

    var body: some View {
        NavigationSplitView {
            List(Page.allCases, selection: $page) { page in
                Label(page.rawValue, systemImage: page.icon).tag(page)
            }
            .navigationSplitViewColumnWidth(180)
        } detail: {
            switch page {
            case .overview: OverviewView()
            case .processes: ProcessesView()
            case .care: CareView()
            }
        }
        .onAppear { monitor.isWindowVisible = true }
        .onDisappear { monitor.isWindowVisible = false }
    }
}

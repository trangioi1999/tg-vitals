import SwiftUI
import VitalsCore

@main
struct TGVitalsApp: App {
    @State private var monitor = SystemMonitor()
    @State private var care = CareModel()

    init() {
        if let directory = Snapshotter.outputDirectory {
            Task { @MainActor in
                try? await Task.sleep(for: .seconds(1))
                await Snapshotter.run(to: directory)
            }
        }
    }

    var body: some Scene {
        MenuBarExtra {
            MenuBarView()
                .environment(monitor)
        } label: {
            // Kept to plain text: the menu bar label re-renders every second.
            Text("⚡︎ \(Format.percent(monitor.cpu?.total ?? 0))")
                .monospacedDigit()
        }
        .menuBarExtraStyle(.window)

        Window("tg-vitals", id: "main") {
            MainView()
                .environment(monitor)
                .environment(care)
                .frame(minWidth: 820, minHeight: 540)
        }
        .defaultSize(width: 980, height: 640)
    }
}

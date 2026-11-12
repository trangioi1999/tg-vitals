import AppKit
import SwiftUI

/// `TGVitals --snapshot <dir>`: renders every page (light + dark) with live data to PNGs, then quits.
/// Used by CI to preview the UI without anyone having to run the app.
@MainActor
enum Snapshotter {
    static var outputDirectory: URL? {
        let args = CommandLine.arguments
        guard let index = args.firstIndex(of: "--snapshot"), index + 1 < args.count else { return nil }
        return URL(fileURLWithPath: args[index + 1], isDirectory: true)
    }

    static func run(to directory: URL) async {
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)

        let monitor = SystemMonitor()
        monitor.isWindowVisible = true
        let care = CareModel()
        care.scan()
        // Let history fill up a bit and the cleanup scan finish.
        try? await Task.sleep(for: .seconds(12))

        let modes: [(String, NSAppearance.Name)] = [("light", .aqua), ("dark", .darkAqua)]
        for (mode, appearance) in modes {
            for page in Page.allCases {
                await capture(
                    MainView(initialPage: page).environment(monitor).environment(care),
                    size: CGSize(width: 980, height: 640), titled: true, appearance: appearance,
                    to: directory.appendingPathComponent("\(page)-\(mode).png")
                )
            }
            await capture(
                MenuBarView().environment(monitor),
                size: CGSize(width: 300, height: 280), titled: false, appearance: appearance,
                to: directory.appendingPathComponent("menubar-\(mode).png")
            )
        }
        NSApp.terminate(nil)
    }

    private static func capture(_ view: some View, size: CGSize, titled: Bool,
                                appearance: NSAppearance.Name, to url: URL) async {
        let controller = NSHostingController(rootView: view.frame(width: size.width, height: size.height))
        controller.sceneBridgingOptions = [.title, .toolbars]

        let style: NSWindow.StyleMask = titled
            ? [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView]
            : [.borderless]
        let window = NSWindow(contentRect: CGRect(origin: CGPoint(x: 80, y: 80), size: size),
                              styleMask: style, backing: .buffered, defer: false)
        window.appearance = NSAppearance(named: appearance)
        window.contentViewController = controller
        window.title = "tg-vitals"
        if titled { window.toolbarStyle = .unified }
        window.setContentSize(size)
        window.makeKeyAndOrderFront(nil)
        try? await Task.sleep(for: .seconds(2))

        // The theme frame includes the title bar / toolbar; the content view alone would not.
        guard let frameView = window.contentView?.superview ?? window.contentView,
              let rep = frameView.bitmapImageRepForCachingDisplay(in: frameView.bounds)
        else { return }
        frameView.cacheDisplay(in: frameView.bounds, to: rep)
        try? rep.representation(using: .png, properties: [:])?.write(to: url)
        window.orderOut(nil)
    }
}

import SwiftUI

@main
struct MdirApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate

    var body: some Scene {
        WindowGroup("mdir") {
            TerminalContainerView()
                .frame(minWidth: 720, minHeight: 460)
                .ignoresSafeArea()
        }
        .windowResizability(.contentMinSize)
        .defaultSize(width: 1000, height: 680)
        .commands {
            // ファイル操作系のデフォルトメニューは TUI 側で完結するため最小限に
            CommandGroup(replacing: .newItem) {}
        }
    }
}

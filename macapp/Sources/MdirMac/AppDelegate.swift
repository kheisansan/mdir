import AppKit

final class AppDelegate: NSObject, NSApplicationDelegate {
    private let menuBar = MenuBarController()

    func applicationDidFinishLaunching(_ notification: Notification) {
        // 常駐アプリだが、Dock アイコンとアプリメニューは従来どおり出す（.accessory にはしない）。
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
        menuBar.install()
    }

    /// ウィンドウを閉じてもアプリは終了せず、メニューバーに常駐し続ける。
    /// 終了手段はメニューバーアイコンの「終了」と Cmd+Q。
    /// mdir 本体を `q` で抜けた場合はウィンドウをしまうだけで常駐は続く。
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
    }

    /// Dock アイコンのクリックでウィンドウへ復帰する。
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows flag: Bool) -> Bool {
        MainWindowManager.shared.show()
        return true
    }

    /// Cmd+Tab などでアクティブになったとき、ウィンドウが隠れたままだと
    /// 何も起きないように見えるため復帰させる。
    /// ステータスメニューを開いただけのアクティブ化では復帰させない。
    func applicationDidBecomeActive(_ notification: Notification) {
        guard !menuBar.isPresentingMenu, !MainWindowManager.shared.isWindowVisible else { return }
        MainWindowManager.shared.show()
    }
}

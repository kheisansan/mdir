import AppKit

/// メニューバー右側（ステータスエリア）に常駐アイコンを表示する。
///
/// - 左クリック: メインウィンドウを表示してアプリをアクティブにする
/// - 右クリック / Control+クリック: 表示・終了のメニューを出す
///
/// ウィンドウを閉じてもアプリは終了しない（AppDelegate 参照）ため、
/// ここが常駐状態から mdir へ復帰する主な入口になる。
final class MenuBarController: NSObject {
    private var statusItem: NSStatusItem?

    /// ステータスメニューを表示している最中かどうか。
    /// ステータスアイテムのクリックはアプリをアクティブにするため、
    /// これを見ないと「メニューを開いただけでウィンドウが出てくる」ことになる。
    private(set) var isPresentingMenu = false

    func install() {
        guard statusItem == nil else { return }

        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        if let button = item.button {
            // 2ペインファイラーであることが分かるテンプレート画像。
            // シンボルを取得できない環境では文字にフォールバックする。
            if let image = NSImage(systemSymbolName: "square.split.2x1", accessibilityDescription: "mdir") {
                image.isTemplate = true
                button.image = image
            } else {
                button.title = "mdir"
            }
            button.toolTip = "mdir"
            button.target = self
            button.action = #selector(statusItemClicked)
            button.sendAction(on: [.leftMouseUp, .rightMouseUp])
        }
        statusItem = item
    }

    @objc private func statusItemClicked() {
        let event = NSApp.currentEvent
        let isSecondaryClick = event?.type == .rightMouseUp
            || event?.modifierFlags.contains(.control) == true
        if isSecondaryClick {
            presentMenu()
        } else {
            MainWindowManager.shared.show()
        }
    }

    private func presentMenu() {
        guard let statusItem else { return }

        let menu = NSMenu()
        let showItem = NSMenuItem(title: "mdir を表示", action: #selector(showWindow), keyEquivalent: "")
        showItem.target = self
        menu.addItem(showItem)
        menu.addItem(.separator())
        let quitItem = NSMenuItem(title: "mdir を終了", action: #selector(quit), keyEquivalent: "q")
        quitItem.target = self
        menu.addItem(quitItem)

        // 一時的に menu を割り当ててからクリックさせるのが、
        // 左クリックの独自アクションとメニュー表示を共存させる定石。
        // performClick 中はメニュー追跡のネストしたループが回るため、
        // その間に届くアクティブ化通知は isPresentingMenu で無視させる。
        isPresentingMenu = true
        statusItem.menu = menu
        statusItem.button?.performClick(nil)
        statusItem.menu = nil
        DispatchQueue.main.async { [weak self] in
            self?.isPresentingMenu = false
        }
    }

    @objc private func showWindow() {
        MainWindowManager.shared.show()
    }

    @objc private func quit() {
        NSApp.terminate(nil)
    }
}

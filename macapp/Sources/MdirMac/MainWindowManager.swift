import AppKit
import SwiftUI

/// SwiftUI の `WindowGroup` が生成したメインウィンドウを AppKit 側から操作するための仲介役。
///
/// 常駐アプリでは閉じるボタン（および Cmd+W）でウィンドウを破棄せず隠すだけにする必要がある。
/// ウィンドウを破棄するとターミナルビューごと解放され、PTY が閉じて mdir 本体プロセスも
/// 終了してしまうため、メニューバーから呼び戻してもカレントディレクトリやマーク状態が失われる。
///
/// SwiftUI は自前の `NSWindowDelegate` を設定しているので、それを保持したまま
/// `windowShouldClose` だけを横取りし、他のメッセージは元のデリゲートへ転送する。
final class MainWindowManager: NSObject, NSWindowDelegate {
    static let shared = MainWindowManager()

    private weak var window: NSWindow?

    /// SwiftUI が設定していた元のデリゲート。
    /// `NSWindow.delegate` は弱参照のため、差し替え後も生存させるべく強参照で保持する。
    private var systemDelegate: NSWindowDelegate?

    private override init() {}

    var isWindowVisible: Bool { window?.isVisible ?? false }

    /// ウィンドウを表示する直前に呼ばれる。終了済みの mdir を起動し直すために
    /// `TerminalContainerView` が登録する。
    var willShow: (() -> Void)?

    /// SwiftUI 側から生成されたウィンドウを受け取り、閉じる動作を横取りする。
    func adopt(_ window: NSWindow) {
        guard self.window !== window else { return }
        self.window = window
        if let existing = window.delegate, existing !== self {
            systemDelegate = existing
        }
        window.delegate = self
    }

    /// ウィンドウを前面に出し、アプリをアクティブにする。
    func show() {
        guard let window else { return }
        willShow?()
        NSApp.activate(ignoringOtherApps: true)
        window.makeKeyAndOrderFront(nil)
    }

    /// ウィンドウをメニューバーへしまう（mdir 本体プロセスは動いたまま）。
    func hide() {
        window?.orderOut(nil)
    }

    // MARK: - NSWindowDelegate

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        sender.orderOut(nil)
        return false
    }

    // MARK: - 元デリゲートへのメッセージ転送

    override func responds(to aSelector: Selector!) -> Bool {
        if super.responds(to: aSelector) { return true }
        return systemDelegate?.responds(to: aSelector) ?? false
    }

    override func forwardingTarget(for aSelector: Selector!) -> Any? {
        guard let systemDelegate, systemDelegate.responds(to: aSelector) else { return nil }
        return systemDelegate
    }
}

/// SwiftUI のビュー階層から、それを載せている `NSWindow` を取り出すためのブリッジ。
/// メインウィンドウの内容に `.background(MainWindowAccessor())` として差し込む。
struct MainWindowAccessor: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        let view = NSView(frame: .zero)
        // ビューがまだウィンドウに載っていないタイミングで呼ばれるため次のループで取得する
        DispatchQueue.main.async {
            if let window = view.window {
                MainWindowManager.shared.adopt(window)
            }
        }
        return view
    }

    func updateNSView(_ nsView: NSView, context: Context) {
        if let window = nsView.window {
            MainWindowManager.shared.adopt(window)
        }
    }
}

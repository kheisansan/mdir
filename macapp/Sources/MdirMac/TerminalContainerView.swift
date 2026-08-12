import SwiftUI
import AppKit
import SwiftTerm

/// SwiftTerm の LocalProcessTerminalView を SwiftUI から扱うためのラッパー。
/// アプリ起動時にバンドル同梱の `mdir` バイナリを擬似端末(PTY)上で起動し、
/// ratatui の描画・マウス・キー操作・色をそのまま表示する。
struct TerminalContainerView: NSViewRepresentable {
    /// `mdir` 本体がモードを通知してくる私的 OSC コード（ime.rs と一致させる）
    static let modeOscCode = 5379
    /// `mdir` 本体がアクティブペインを通知してくる私的 OSC コード（ime.rs と一致させる）
    static let activePaneOscCode = 5380

    @EnvironmentObject private var controller: MdirController

    func makeCoordinator() -> Coordinator {
        Coordinator()
    }

    func makeNSView(context: Context) -> MdirTerminalView {
        let terminal = MdirTerminalView(
            frame: NSRect(x: 0, y: 0, width: 1000, height: 680)
        )
        terminal.processDelegate = context.coordinator
        controller.attach(terminal: terminal)

        // 初期フォントは OS 標準サイズ（= 最小サイズ）。
        // 以降は Cmd + "+"/"-" で拡大・縮小できる（MdirTerminalView 側で処理）。
        terminal.applyDefaultFont()

        // mdir 本体からのモード通知 OSC を受け取り、IME バイパスを切り替える。
        // ペイロード "1" = テキスト入力（IME 許可）、"0" = ナビゲーション（ASCII 強制）。
        // OSC ハンドラは受信スレッドで呼ばれるため UI 更新は main へ回す。
        terminal.getTerminal().registerOscHandler(code: Self.modeOscCode) { [weak terminal] data in
            let allowIme = data.first == UInt8(ascii: "1")
            DispatchQueue.main.async {
                terminal?.imeAllowed = allowIme
            }
        }

        // mdir 本体からのアクティブペイン通知 OSC を受け取り、メニューバー/
        // サイドバーがペイン指定操作の前に Tab 送出要否を判断できるようにする。
        // ペイロード "L" = 左ペイン、"R" = 右ペイン。
        terminal.getTerminal().registerOscHandler(code: Self.activePaneOscCode) { [weak controller] data in
            guard let raw = data.first, let side = PaneSide(rawValue: String(UnicodeScalar(raw))) else { return }
            DispatchQueue.main.async {
                controller?.handleActivePaneReport(side)
            }
        }

        // ウィンドウを開くたびに、mdir が終了済みなら新しいセッションを立ち上げ直す。
        // 常駐アプリでは `q` で mdir を抜けてもアプリ自体は生き続けるため。
        MainWindowManager.shared.willShow = { [weak terminal] in
            terminal?.relaunchMdirIfNeeded()
        }

        MdirProcess.start(in: terminal)
        return terminal
    }

    func updateNSView(_ nsView: MdirTerminalView, context: Context) {}

    final class Coordinator: NSObject, LocalProcessTerminalViewDelegate {
        // MARK: - LocalProcessTerminalViewDelegate

        func sizeChanged(source: LocalProcessTerminalView, newCols: Int, newRows: Int) {}

        func setTerminalTitle(source: LocalProcessTerminalView, title: String) {
            source.window?.title = title.isEmpty ? "mdir" : title
        }

        func hostCurrentDirectoryUpdate(source: TerminalView, directory: String?) {}

        func processTerminated(source: TerminalView, exitCode: Int32?) {
            // 常駐アプリなので mdir を抜けても（:q や q）アプリは終了させず、
            // ウィンドウをメニューバーへしまうだけにする。
            // 次にウィンドウを開くとき MainWindowManager.willShow から mdir が起動し直される。
            DispatchQueue.main.async {
                MainWindowManager.shared.hide()
            }
        }
    }
}

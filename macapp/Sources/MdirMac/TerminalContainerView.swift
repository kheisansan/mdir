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

        context.coordinator.launchMdir(in: terminal)
        return terminal
    }

    func updateNSView(_ nsView: MdirTerminalView, context: Context) {}

    final class Coordinator: NSObject, LocalProcessTerminalViewDelegate {
        private var didLaunch = false

        /// 同梱 `mdir` バイナリを解決して PTY 上で起動する。
        func launchMdir(in terminal: LocalProcessTerminalView) {
            guard !didLaunch else { return }
            didLaunch = true

            let binary = Self.resolveMdirBinary()

            // GUI から起動すると cwd が "/" になるため、ホームを起点にする。
            FileManager.default.changeCurrentDirectoryPath(NSHomeDirectory())

            var env = Terminal.getEnvironmentVariables(termName: "xterm-256color", trueColor: true)
            // ネイティブアプリ埋め込みであることを mdir 本体へ伝える。
            // これを受けて mdir は TIS による入力ソース切替を行わず、
            // モード変化を OSC で通知するだけになる（IME 制御は Swift 側が担う）。
            env.append("MDIR_HOST=macapp")

            terminal.startProcess(
                executable: binary,
                args: [],
                environment: env,
                execName: nil
            )
        }

        /// 実行する `mdir` のパスを決定する。
        /// 1. 環境変数 MDIR_BIN（開発時の上書き用）
        /// 2. .app バンドル内 Contents/Resources/mdir
        /// 3. リポジトリの dirtools/target/release/mdir（`swift run` 開発時）
        /// 4. PATH 上の mdir
        private static func resolveMdirBinary() -> String {
            let fm = FileManager.default

            if let override = ProcessInfo.processInfo.environment["MDIR_BIN"],
               fm.isExecutableFile(atPath: override) {
                return override
            }

            if let bundled = Bundle.main.path(forResource: "mdir", ofType: nil),
               fm.isExecutableFile(atPath: bundled) {
                return bundled
            }

            // 開発時フォールバック: 実行ファイルからリポジトリ相対で探す
            let exeDir = Bundle.main.bundleURL.deletingLastPathComponent()
            let devCandidates = [
                exeDir.appendingPathComponent("../../../dirtools/target/release/mdir").path,
                exeDir.appendingPathComponent("../../../../dirtools/target/release/mdir").path
            ]
            for candidate in devCandidates where fm.isExecutableFile(atPath: candidate) {
                return candidate
            }

            for dir in ["/usr/local/bin", "/opt/homebrew/bin"] {
                let candidate = "\(dir)/mdir"
                if fm.isExecutableFile(atPath: candidate) {
                    return candidate
                }
            }

            // 最終フォールバック（見つからなければ起動時にエラー表示される）
            return "mdir"
        }

        // MARK: - LocalProcessTerminalViewDelegate

        func sizeChanged(source: LocalProcessTerminalView, newCols: Int, newRows: Int) {}

        func setTerminalTitle(source: LocalProcessTerminalView, title: String) {
            source.window?.title = title.isEmpty ? "mdir" : title
        }

        func hostCurrentDirectoryUpdate(source: TerminalView, directory: String?) {}

        func processTerminated(source: TerminalView, exitCode: Int32?) {
            // mdir を抜けたら（:q や q）アプリも終了する
            DispatchQueue.main.async {
                NSApplication.shared.terminate(nil)
            }
        }
    }
}

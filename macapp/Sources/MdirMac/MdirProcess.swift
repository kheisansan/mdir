import AppKit
import SwiftTerm

/// 同梱の `mdir` バイナリを解決し、擬似端末 (PTY) 上で起動する処理。
///
/// 常駐アプリでは mdir を `q` で抜けたあとに再起動することがあるため、
/// ターミナルビュー生成時の初回起動と再起動の両方から使えるよう独立させている。
enum MdirProcess {
    static func start(in terminal: LocalProcessTerminalView) {
        // GUI から起動すると cwd が "/" になるため、ホームを起点にする。
        FileManager.default.changeCurrentDirectoryPath(NSHomeDirectory())

        var env = Terminal.getEnvironmentVariables(termName: "xterm-256color", trueColor: true)
        // ネイティブアプリ埋め込みであることを mdir 本体へ伝える。
        // これを受けて mdir は TIS による入力ソース切替を行わず、
        // モード変化を OSC で通知するだけになる（IME 制御は Swift 側が担う）。
        env.append("MDIR_HOST=macapp")

        terminal.startProcess(
            executable: resolveBinary(),
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
    private static func resolveBinary() -> String {
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
}

import AppKit
import Combine

/// mdir 本体（埋め込みターミナル）のアクティブペイン。
/// dirtools 側の `PaneSide` と対応し、OSC 通知の "L"/"R" と一致させる。
enum PaneSide: String {
    case left = "L"
    case right = "R"
}

/// メニューバー・サイドバー・ミラーコピーパネルなど、ネイティブ UI 側からの
/// すべての操作をとりまとめる中心的コントローラー。
///
/// mdir 本体は擬似端末 (PTY) 上で動く独立プロセスであり、Swift 側から
/// その内部状態（カーソル位置・マーク・ペイン構成等）を直接操作することはできない。
/// そのためネイティブ UI からの操作はすべて「対応するキー入力/コマンドを
/// ターミナルへ送出する」ことで実現する（メニューバーもサイドバーも同じ経路）。
///
/// アクティブペイン（左/右）は mdir 本体から私的 OSC で通知される
/// （TerminalContainerView が受信して `activePane` を更新する）。
/// サイドバー/メニューから特定ペインを指定した操作を行う際は、
/// 必要なら Tab を送ってアクティブペインを切り替えてから本来のコマンドを送る。
@MainActor
final class MdirController: ObservableObject {
    /// mdir 本体側が現在アクティブと報告しているペイン
    @Published private(set) var activePane: PaneSide = .left

    /// 現在のターミナルフォントサイズ。サイドバー/ミラーコピーパネルの
    /// SwiftUI 側フォントもこれに追従させる（`MdirTerminalView.onFontSizeChange` 経由）。
    @Published private(set) var fontSize: CGFloat = NSFont.systemFontSize

    /// コピー元/コピー先フォルダとその履歴・お気に入り（永続化）
    @Published var sourceFolder: String = ""
    @Published var destFolder: String = ""
    @Published var sourceFolders = FolderListManager()
    @Published var destFolders = FolderListManager()

    weak var terminal: MdirTerminalView?

    private var config = MacAppConfig.load()
    private var hotkeys: GlobalHotkeys?
    private var mirrorRunning = false

    init() {
        sourceFolder = config.sourceFolder ?? ""
        destFolder = config.destFolder ?? ""
        sourceFolders.history = config.sourceHistory
        sourceFolders.favorites = config.sourceFavorites
        destFolders.history = config.destHistory
        destFolders.favorites = config.destFavorites
    }

    func attach(terminal: MdirTerminalView) {
        self.terminal = terminal
        terminal.onFontSizeChange = { [weak self] size in
            self?.fontSize = size
        }
    }

    /// 起動後、ウィンドウが表示されてから一度だけ呼ぶ（グローバルショートカット登録）。
    func installGlobalHotkeys() {
        guard hotkeys == nil else { return }
        hotkeys = GlobalHotkeys(
            onMirrorForward: { [weak self] in self?.runMirror(forward: true) },
            onMirrorBackward: { [weak self] in self?.runMirror(forward: false) }
        )
        if hotkeys?.forwardRegistered != true || hotkeys?.backwardRegistered != true {
            ToastPanel.show(
                title: "ショートカット登録失敗",
                body: "グローバルショートカットの一部を登録できませんでした。他のアプリと競合している可能性があります。",
                isError: true, seconds: 5)
        }
    }

    // MARK: - OSC 通知の受信

    /// mdir 本体からアクティブペイン変化の通知を受けたときに呼ぶ。
    func handleActivePaneReport(_ side: PaneSide) {
        activePane = side
    }

    // MARK: - 低レベル送出

    private func sendRaw(_ text: String) {
        terminal?.send(txt: text)
    }

    /// ノーマルモードのキーを1つ送る（例: "c", "d", "z"）。
    func sendKey(_ key: String) {
        sendRaw(key)
    }

    /// `:` コマンドモードでコマンドを実行する（例で "sort name" → ":sort name\r"）。
    /// 呼び出し前に必ずノーマルモードであることを仮定する（メニュー/サイドバー操作の既知の制約）。
    func sendCommand(_ body: String) {
        sendRaw(":\(body)\r")
    }

    /// 指定ペインが非アクティブなら Tab を送って切り替える。
    /// mdir 本体からの OSC 応答を待たず、楽観的にローカル状態も更新する
    /// （同一プロセス内 PTY 経由のため往復遅延はごく僅かだが、連続操作時の
    /// 二重切り替えを避けるため）。
    func ensureActive(_ side: PaneSide) {
        guard activePane != side else { return }
        sendRaw("\t")
        activePane = side
    }

    // MARK: - ナビゲーション / 操作（メニューバー用）

    func goParent() { sendKey("h") }
    func goHome() { sendKey("~") }
    func activate(_ side: PaneSide) { ensureActive(side) }
    func copyToOtherPane() { sendKey("c") }
    func moveToOtherPane() { sendKey("m") }
    func delete() { sendKey("d") }
    func rename() { sendKey("r") }
    func newFolder() { sendKey("n") }
    func newFile() { sendKey("t") }
    func toggleHidden() { sendKey("z") }
    func fileInfo() { sendKey("i") }
    func gitPull() { sendKey("y") }
    func copyFileName() { sendKey("1") }
    func copyFullPath() { sendKey("2") }
    func refresh() { sendRaw("\u{12}") } // Ctrl+R
    func showHelp() { sendCommand("help") }

    func setSort(_ criteria: String) { sendCommand("sort \(criteria)") }

    // MARK: - フォント

    func increaseFont() { terminal?.increaseFontSize() }
    func decreaseFont() { terminal?.decreaseFontSize() }
    func resetFont() { terminal?.resetToDefaultFontSize() }

    // MARK: - サイドバー（フォルダツリー）からのナビゲーション

    /// サイドバーで選択されたフォルダへ、指定ペインを移動する。
    func navigate(_ side: PaneSide, to path: String) {
        ensureActive(side)
        sendCommand("cd \(path)")
    }

    // MARK: - コピー元 / コピー先（ミラーコピーパネル）

    func setSourceFolder(_ path: String, addToHistory: Bool = true) {
        sourceFolder = path
        config.sourceFolder = path
        if addToHistory {
            sourceFolders.addToHistory(path)
            persistFolderLists()
        }
        config.save()
    }

    func setDestFolder(_ path: String, addToHistory: Bool = true) {
        destFolder = path
        config.destFolder = path
        if addToHistory {
            destFolders.addToHistory(path)
            persistFolderLists()
        }
        config.save()
    }

    func addFavorite(isSource: Bool, path: String) {
        if isSource {
            sourceFolders.addToFavorite(path)
        } else {
            destFolders.addToFavorite(path)
        }
        persistFolderLists()
        ToastPanel.show(title: "お気に入り", body: "登録しました:\n\(path)", seconds: 2)
    }

    func removeFavorite(isSource: Bool, path: String) {
        if isSource {
            sourceFolders.removeFromFavorites(path)
        } else {
            destFolders.removeFromFavorites(path)
        }
        persistFolderLists()
    }

    private func persistFolderLists() {
        config.sourceHistory = sourceFolders.history
        config.sourceFavorites = sourceFolders.favorites
        config.destHistory = destFolders.history
        config.destFavorites = destFolders.favorites
        config.save()
        objectWillChange.send()
    }

    /// コピー元/コピー先フォルダを強制上書きミラーコピーする。
    /// forward: true なら コピー元→コピー先、false ならその逆。
    func runMirror(forward: Bool) {
        if mirrorRunning {
            ToastPanel.show(title: "実行中", body: "前回のコピーがまだ実行中です。", isError: true)
            return
        }
        let source = forward ? sourceFolder : destFolder
        let dest = forward ? destFolder : sourceFolder
        let label = forward ? "コピー元 → コピー先" : "コピー先 → コピー元"

        guard !source.isEmpty, !dest.isEmpty else {
            ToastPanel.show(
                title: "設定不足",
                body: "サイドバーでコピー元とコピー先のフォルダを設定してください。",
                isError: true, seconds: 5)
            return
        }

        mirrorRunning = true
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let result = MirrorFileOperations.mirrorCopy(source: source, dest: dest)
            DispatchQueue.main.async {
                guard let self else { return }
                self.mirrorRunning = false
                if !result.errors.isEmpty {
                    let detail = result.errors.prefix(3).joined(separator: "\n")
                    ToastPanel.show(
                        title: "\(label): 一部失敗",
                        body: "\(result.filesCopied) 件コピー / \(result.errors.count) 件失敗\n\(detail)",
                        isError: true, seconds: 6)
                } else {
                    ToastPanel.show(
                        title: label,
                        body: "\(result.filesCopied) ファイルを上書きコピーしました\n\(source)\n→ \(dest)")
                }
                self.refresh()
            }
        }
    }

    func browseFolder(title: String, initial: String) -> String? {
        let panel = NSOpenPanel()
        panel.title = title
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.canCreateDirectories = true
        if !initial.isEmpty {
            panel.directoryURL = URL(fileURLWithPath: initial, isDirectory: true)
        }
        return panel.runModal() == .OK ? panel.url?.path : nil
    }
}

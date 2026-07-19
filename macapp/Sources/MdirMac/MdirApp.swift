import SwiftUI

@main
struct MdirApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var controller = MdirController()

    var body: some Scene {
        WindowGroup("mdir") {
            MdirMainView()
                .environmentObject(controller)
                .frame(minWidth: 900, minHeight: 520)
                .onAppear { controller.installGlobalHotkeys() }
        }
        .windowResizability(.contentMinSize)
        .defaultSize(width: 1280, height: 780)
        .commands {
            // ファイル操作系のデフォルトメニューは TUI 側で完結するため最小限に
            CommandGroup(replacing: .newItem) {}

            CommandGroup(after: .pasteboard) {
                Divider()
                Button("フォルダ名をコピー (1)") { controller.copyFileName() }
                Button("フルパスをコピー (2)") { controller.copyFullPath() }
            }

            CommandGroup(after: .toolbar) {
                Divider()
                Button("フォントを拡大") { controller.increaseFont() }
                    .keyboardShortcut("+", modifiers: .command)
                Button("フォントを縮小") { controller.decreaseFont() }
                    .keyboardShortcut("-", modifiers: .command)
                Button("フォントをリセット") { controller.resetFont() }
                    .keyboardShortcut("0", modifiers: .command)
            }

            CommandMenu("ナビゲーション") {
                Button("親フォルダへ (h)") { controller.goParent() }
                Button("ホームフォルダへ (~)") { controller.goHome() }
                Divider()
                Button("左ペインをアクティブに") { controller.activate(.left) }
                Button("右ペインをアクティブに") { controller.activate(.right) }
            }

            CommandMenu("操作") {
                Button("反対ペインへコピー (c)") { controller.copyToOtherPane() }
                Button("反対ペインへ移動 (m)") { controller.moveToOtherPane() }
                Button("削除 (d)") { controller.delete() }
                Button("名前の変更 (r)") { controller.rename() }
                Button("新規フォルダ (n)") { controller.newFolder() }
                Button("新規ファイル (t)") { controller.newFile() }
                Divider()
                Menu("ソート") {
                    Button("名前") { controller.setSort("name") }
                    Button("サイズ") { controller.setSort("size") }
                    Button("更新日時") { controller.setSort("date") }
                    Button("拡張子") { controller.setSort("ext") }
                }
                Button("隠しファイル表示切替 (z)") { controller.toggleHidden() }
                Button("ファイル情報 (i)") { controller.fileInfo() }
                Button("Git Pull (y)") { controller.gitPull() }
                Divider()
                Button("最新の情報に更新") { controller.refresh() }
                    .keyboardShortcut("r", modifiers: .command)
            }

            CommandMenu("同期") {
                Button("コピー元 → コピー先（強制上書き）") { controller.runMirror(forward: true) }
                Button("コピー先 → コピー元（強制上書き）") { controller.runMirror(forward: false) }
                Divider()
                Text("グローバルショートカット: ⌃⌥⌘[ / ⌃⌥⌘]")
            }

            CommandGroup(replacing: .help) {
                Button("キーバインド一覧") { controller.showHelp() }
            }
        }
    }
}

/// メインウィンドウのレイアウト: 左サイドバー + ターミナル + 右サイドバー、
/// その下にコピー元/コピー先のミラーコピーパネル。
/// winapp の MainWindow.xaml（ツリー2列 + 2ペイン + 下部ミラーパネル）に対応。
private struct MdirMainView: View {
    var body: some View {
        VStack(spacing: 0) {
            HStack(spacing: 0) {
                SidebarFolderTreeView(side: .left, title: "フォルダツリー（左ペイン）")
                    .frame(width: 220)
                Divider()
                TerminalContainerView()
                    .frame(minWidth: 480, minHeight: 320)
                Divider()
                SidebarFolderTreeView(side: .right, title: "フォルダツリー（右ペイン）")
                    .frame(width: 220)
            }
            Divider()
            MirrorSyncPanel()
                .frame(height: 190)
        }
        .ignoresSafeArea()
    }
}

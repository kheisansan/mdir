import SwiftUI
import AppKit

/// 遅延展開するフォルダツリーの1ノード。
/// winapp/Controls/FolderTreeControl.xaml(.cs) の SwiftUI 移植（ドライブ→ボリュームに読み替え）。
final class FolderNode: Identifiable, ObservableObject {
    let path: String
    let name: String
    let icon: String
    let isRoot: Bool

    private var loadedChildren: [FolderNode]?

    var id: String { path }

    init(path: String, name: String, icon: String = "\u{1F4C1}", isRoot: Bool = false) {
        self.path = path
        self.name = name
        self.icon = icon
        self.isRoot = isRoot
    }

    /// `OutlineGroup` から参照される遅延ロードの子ノード一覧。
    /// 未読み込みなら初回アクセス時に同期的にディレクトリを読み込みキャッシュする。
    var children: [FolderNode]? {
        if let loadedChildren { return loadedChildren.isEmpty ? nil : loadedChildren }
        let loaded = Self.loadChildren(of: path)
        loadedChildren = loaded
        return loaded.isEmpty ? nil : loaded
    }

    /// フォルダ内容が変わった可能性がある場合に呼び、次回アクセス時に再読み込みさせる。
    func invalidate() {
        loadedChildren = nil
    }

    private static func loadChildren(of path: String) -> [FolderNode] {
        let fm = FileManager.default
        guard let entries = try? fm.contentsOfDirectory(atPath: path) else { return [] }
        return entries
            .filter { !$0.hasPrefix(".") }
            .sorted { $0.localizedStandardCompare($1) == .orderedAscending }
            .compactMap { name -> FolderNode? in
                let full = (path as NSString).appendingPathComponent(name)
                var isDir: ObjCBool = false
                guard fm.fileExists(atPath: full, isDirectory: &isDir), isDir.boolValue else { return nil }
                return FolderNode(path: full, name: name)
            }
    }
}

/// 左/右ペイン用のフォルダツリーサイドバー。
/// クリックで対応するペインを指定パスへ移動させる（`MdirController.navigate(_:to:)`）。
struct SidebarFolderTreeView: View {
    let side: PaneSide
    let title: String
    @EnvironmentObject private var controller: MdirController
    @State private var roots: [FolderNode] = []
    @State private var selection: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(title)
                .font(.system(size: controller.fontSize, weight: .bold))
                .padding(.horizontal, 8)
                .padding(.top, 8)
                .padding(.bottom, 4)

            List(selection: $selection) {
                OutlineGroup(roots, children: \.children) { node in
                    Label(node.name, systemImage: node.isRoot ? "internaldrive" : "folder")
                        .font(.system(size: controller.fontSize))
                        .lineLimit(1)
                        .truncationMode(.middle)
                        .tag(node.path)
                        .contextMenu { contextMenu(for: node) }
                }
            }
            .listStyle(.sidebar)
        }
        .onAppear(perform: loadRoots)
        .onChange(of: selection) { newValue in
            guard let newValue else { return }
            controller.navigate(side, to: newValue)
        }
    }

    private func loadRoots() {
        var result: [FolderNode] = []
        let home = FileManager.default.homeDirectoryForCurrentUser.path
        result.append(FolderNode(path: home, name: "ホーム", icon: "\u{1F3E0}", isRoot: true))

        // ホームディレクトリより上位（/Users, / 等）へもたどれるよう、
        // 起動ボリューム自体もルートとして表示する（WPF版のドライブ一覧に相当）。
        let rootVolumeName = (try? URL(fileURLWithPath: "/").resourceValues(forKeys: [.volumeLocalizedNameKey]))?
            .volumeLocalizedName ?? "/"
        result.append(FolderNode(path: "/", name: rootVolumeName, icon: "\u{1F4BB}", isRoot: true))

        let rootDevice = Self.deviceID(of: "/")
        if let volumes = try? FileManager.default.contentsOfDirectory(atPath: "/Volumes") {
            for name in volumes.sorted(by: { $0.localizedStandardCompare($1) == .orderedAscending }) {
                let full = "/Volumes/\(name)"
                var isDir: ObjCBool = false
                guard FileManager.default.fileExists(atPath: full, isDirectory: &isDir), isDir.boolValue else { continue }
                // 起動ボリュームは /Volumes 配下にも（ファームリンク経由で）現れることがあるため、
                // 上で追加済みの "/" と重複表示しないよう同一デバイスならスキップする。
                if let rootDevice, Self.deviceID(of: full) == rootDevice { continue }
                result.append(FolderNode(path: full, name: name, icon: "\u{1F4BE}", isRoot: true))
            }
        }
        roots = result
    }

    private static func deviceID(of path: String) -> dev_t? {
        var st = stat()
        guard stat(path, &st) == 0 else { return nil }
        return st.st_dev
    }

    @ViewBuilder
    private func contextMenu(for node: FolderNode) -> some View {
        Button("サブフォルダを作成…") { createSubfolder(in: node) }
        Button("名前の変更…") { rename(node) }
            .disabled(node.isRoot)
        Button("ゴミ箱に入れる") { delete(node) }
            .disabled(node.isRoot)
        Divider()
        Button("フォルダ名をコピー") { copyToPasteboard(node.name) }
        Button("フルパスをコピー") { copyToPasteboard(node.path) }
    }

    private func createSubfolder(in node: FolderNode) {
        guard let name = TextPrompt.ask(title: "サブフォルダの作成", message: "フォルダ名:", defaultValue: "新しいフォルダ"),
              !name.isEmpty else { return }
        let target = uniquePath(base: (node.path as NSString).appendingPathComponent(name))
        do {
            try FileManager.default.createDirectory(atPath: target, withIntermediateDirectories: true)
            reload(node)
        } catch {
            ToastPanel.show(title: "フォルダ作成エラー", body: error.localizedDescription, isError: true)
        }
    }

    private func rename(_ node: FolderNode) {
        guard let newName = TextPrompt.ask(title: "フォルダ名の変更", message: "新しい名前:", defaultValue: node.name),
              !newName.isEmpty, newName != node.name else { return }
        let parent = (node.path as NSString).deletingLastPathComponent
        let target = (parent as NSString).appendingPathComponent(newName)
        do {
            try FileManager.default.moveItem(atPath: node.path, toPath: target)
            loadRoots()
            controller.refresh()
        } catch {
            ToastPanel.show(title: "名前の変更エラー", body: error.localizedDescription, isError: true)
        }
    }

    private func delete(_ node: FolderNode) {
        let alert = NSAlert()
        alert.messageText = "フォルダをゴミ箱に移動しますか？"
        alert.informativeText = node.path
        alert.addButton(withTitle: "ゴミ箱に入れる")
        alert.addButton(withTitle: "キャンセル")
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        do {
            try FileManager.default.trashItem(at: URL(fileURLWithPath: node.path), resultingItemURL: nil)
            loadRoots()
            controller.refresh()
        } catch {
            ToastPanel.show(title: "削除エラー", body: error.localizedDescription, isError: true)
        }
    }

    private func copyToPasteboard(_ text: String) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }

    private func reload(_ node: FolderNode) {
        node.invalidate()
        // OutlineGroup に再読み込みを促すため、ルート配列を作り直す
        roots = roots
        controller.refresh()
    }

    private func uniquePath(base: String) -> String {
        let fm = FileManager.default
        guard fm.fileExists(atPath: base) else { return base }
        var n = 2
        while true {
            let candidate = "\(base) (\(n))"
            if !fm.fileExists(atPath: candidate) { return candidate }
            n += 1
        }
    }
}

/// 簡易な1行テキスト入力ダイアログ（winapp/Dialogs/InputDialog.xaml 相当）。
enum TextPrompt {
    @MainActor
    static func ask(title: String, message: String, defaultValue: String) -> String? {
        let alert = NSAlert()
        alert.messageText = title
        alert.informativeText = message
        alert.addButton(withTitle: "OK")
        alert.addButton(withTitle: "キャンセル")

        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 260, height: 24))
        field.stringValue = defaultValue
        alert.accessoryView = field
        alert.window.initialFirstResponder = field

        return alert.runModal() == .alertFirstButtonReturn ? field.stringValue : nil
    }
}

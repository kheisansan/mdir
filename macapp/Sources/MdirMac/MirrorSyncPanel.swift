import SwiftUI

/// コピー元/コピー先フォルダの選択・お気に入り・履歴・強制上書きミラーコピー実行パネル。
/// winapp の MainWindow.xaml 下部（コピー元/コピー先グループボックス）の SwiftUI 移植。
struct MirrorSyncPanel: View {
    @EnvironmentObject private var controller: MdirController

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            FolderSlot(
                title: "コピー元のフォルダ",
                path: controller.sourceFolder,
                folders: controller.sourceFolders,
                isSource: true)
            FolderSlot(
                title: "コピー先のフォルダ",
                path: controller.destFolder,
                folders: controller.destFolders,
                isSource: false)
        }
        .padding(8)
        .frame(maxWidth: .infinity)
        .background(.regularMaterial)
        .overlay(alignment: .top) { Divider() }
    }
}

private struct FolderSlot: View {
    let title: String
    let path: String
    let folders: FolderListManager
    let isSource: Bool
    @EnvironmentObject private var controller: MdirController

    private var fontSize: CGFloat { controller.fontSize }
    private var smallFontSize: CGFloat { max(9, fontSize - 2) }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title).font(.system(size: fontSize, weight: .semibold))

            HStack {
                Text(path.isEmpty ? "未設定" : path)
                    .font(.system(size: fontSize))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .foregroundStyle(path.isEmpty ? .secondary : .primary)
                Spacer()
                Button("Browse…") { browse() }
                    .font(.system(size: fontSize))
            }

            if !folders.favorites.isEmpty {
                Text("★ お気に入り").font(.system(size: smallFontSize)).foregroundStyle(.secondary)
                folderList(folders.favorites, isFavorite: true)
            }
            if !folders.history.isEmpty {
                Text("履歴").font(.system(size: smallFontSize)).foregroundStyle(.secondary)
                folderList(folders.history, isFavorite: false)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder
    private func folderList(_ items: [String], isFavorite: Bool) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(items, id: \.self) { item in
                Text(item)
                    .font(.system(size: fontSize))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .padding(.vertical, 2)
                    .padding(.horizontal, 4)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                    .onTapGesture { select(item) }
                    .contextMenu {
                        Button(isSource ? "コピー元に設定" : "コピー先に設定") { select(item) }
                        if isFavorite {
                            Button("お気に入りから削除") {
                                controller.removeFavorite(isSource: isSource, path: item)
                            }
                        } else {
                            Button("お気に入りに追加") {
                                controller.addFavorite(isSource: isSource, path: item)
                            }
                        }
                    }
            }
        }
        .frame(maxHeight: max(48, fontSize * 5.5))
    }

    private func select(_ path: String) {
        if isSource {
            controller.setSourceFolder(path, addToHistory: false)
        } else {
            controller.setDestFolder(path, addToHistory: false)
        }
    }

    private func browse() {
        guard let picked = controller.browseFolder(
            title: isSource ? "コピー元のフォルダを選択" : "コピー先のフォルダを選択",
            initial: path
        ) else { return }
        if isSource {
            controller.setSourceFolder(picked)
        } else {
            controller.setDestFolder(picked)
        }
    }
}

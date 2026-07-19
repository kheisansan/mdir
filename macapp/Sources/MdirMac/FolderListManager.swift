import Foundation

/// コピー元・先フォルダの履歴とお気に入り（各最大20件）を管理する。
/// winapp（WPF版）の `FolderListManager` と同じ仕様の Swift 移植。
struct FolderListManager {
    static let maxItems = 20

    var history: [String] = []
    var favorites: [String] = []

    mutating func addToHistory(_ path: String?) {
        guard let path, !path.isEmpty, isDirectory(path) else { return }
        let full = normalize(path)
        history.removeAll { $0.caseInsensitiveCompare(full) == .orderedSame }
        history.insert(full, at: 0)
        if history.count > Self.maxItems {
            history.removeLast(history.count - Self.maxItems)
        }
    }

    mutating func addToFavorite(_ path: String?) {
        guard let path, !path.isEmpty, isDirectory(path) else { return }
        let full = normalize(path)
        favorites.removeAll { $0.caseInsensitiveCompare(full) == .orderedSame }
        favorites.insert(full, at: 0)
        if favorites.count > Self.maxItems {
            favorites.removeLast(favorites.count - Self.maxItems)
        }
    }

    mutating func removeFromFavorites(_ path: String) {
        favorites.removeAll { $0.caseInsensitiveCompare(path) == .orderedSame }
    }

    func isFavorite(_ path: String) -> Bool {
        favorites.contains { $0.caseInsensitiveCompare(path) == .orderedSame }
    }

    private func isDirectory(_ path: String) -> Bool {
        var isDir: ObjCBool = false
        return FileManager.default.fileExists(atPath: path, isDirectory: &isDir) && isDir.boolValue
    }

    private func normalize(_ path: String) -> String {
        (path as NSString).standardizingPath
    }
}

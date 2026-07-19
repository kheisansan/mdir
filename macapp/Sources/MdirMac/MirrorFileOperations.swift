import Foundation

/// winapp（WPF版）の `MdirWin.Core.FileOperations.MirrorCopy` の Swift 移植。
/// コピー元フォルダの内容をコピー先フォルダへ強制上書きコピーする
/// （読み取り専用フラグも解除して上書きする）。
enum MirrorFileOperations {
    struct Result {
        var filesCopied = 0
        var directoriesCreated = 0
        var errors: [String] = []
    }

    static func mirrorCopy(source sourceDir: String, dest destDir: String) -> Result {
        let fm = FileManager.default
        let source = (sourceDir as NSString).resolvingSymlinksInPath
        let dest = (destDir as NSString).resolvingSymlinksInPath
        var result = Result()

        var isDir: ObjCBool = false
        guard fm.fileExists(atPath: source, isDirectory: &isDir), isDir.boolValue else {
            result.errors.append("コピー元フォルダが見つかりません: \(source)")
            return result
        }
        if pathsEqual(source, dest) {
            result.errors.append("コピー元とコピー先が同じフォルダです。")
            return result
        }
        if isInside(dest, of: source) {
            result.errors.append("コピー先がコピー元の中にあるためコピーできません。")
            return result
        }

        try? fm.createDirectory(atPath: dest, withIntermediateDirectories: true)

        guard let enumerator = fm.enumerator(
            at: URL(fileURLWithPath: source, isDirectory: true),
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsPackageDescendants]
        ) else {
            return result
        }

        for case let url as URL in enumerator {
            let relative = String(url.path.dropFirst(source.count)).trimmingCharacters(in: CharacterSet(charactersIn: "/"))
            guard !relative.isEmpty else { continue }
            let targetPath = (dest as NSString).appendingPathComponent(relative)

            let isDirectory = (try? url.resourceValues(forKeys: [.isDirectoryKey]))?.isDirectory ?? false
            if isDirectory {
                if !fm.fileExists(atPath: targetPath) {
                    do {
                        try fm.createDirectory(atPath: targetPath, withIntermediateDirectories: true)
                        result.directoriesCreated += 1
                    } catch {
                        result.errors.append("\(targetPath): \(error.localizedDescription)")
                    }
                }
            } else {
                do {
                    try forceCopyFile(from: url.path, to: targetPath)
                    result.filesCopied += 1
                } catch {
                    result.errors.append("\(url.path): \(error.localizedDescription)")
                }
            }
        }

        return result
    }

    /// 読み取り専用属性が付いていても強制的に上書きコピーする。
    static func forceCopyFile(from sourceFile: String, to destFile: String) throws {
        let fm = FileManager.default
        let parent = (destFile as NSString).deletingLastPathComponent
        if !parent.isEmpty {
            try? fm.createDirectory(atPath: parent, withIntermediateDirectories: true)
        }
        if fm.fileExists(atPath: destFile) {
            try? fm.setAttributes([.immutable: false], ofItemAtPath: destFile)
            try fm.removeItem(atPath: destFile)
        }
        try fm.copyItem(atPath: sourceFile, toPath: destFile)
    }

    private static func pathsEqual(_ a: String, _ b: String) -> Bool {
        trimTrailingSlash(a).caseInsensitiveCompare(trimTrailingSlash(b)) == .orderedSame
    }

    /// path が directory の内側（またはそれ自身）にあるかを判定する。
    private static func isInside(_ path: String, of directory: String) -> Bool {
        let p = trimTrailingSlash(path)
        let d = trimTrailingSlash(directory)
        return p.lowercased().hasPrefix(d.lowercased() + "/")
    }

    private static func trimTrailingSlash(_ path: String) -> String {
        var p = path
        while p.count > 1, p.hasSuffix("/") {
            p.removeLast()
        }
        return p
    }
}

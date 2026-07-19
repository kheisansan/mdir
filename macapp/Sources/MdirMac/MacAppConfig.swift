import Foundation

/// Mac アプリの設定永続化（`~/Library/Application Support/mdir/mac_config.json`）。
/// winapp（WPF版）の `AppConfig` に相当。左右ペインの操作は mdir 本体
/// （dirtools）自身の状態なのでここでは扱わず、ミラーコピー用の
/// コピー元/コピー先フォルダとその履歴・お気に入りのみを保持する。
struct MacAppConfig: Codable {
    var sourceFolder: String?
    var destFolder: String?
    var sourceHistory: [String] = []
    var destHistory: [String] = []
    var sourceFavorites: [String] = []
    var destFavorites: [String] = []

    private static var configURL: URL {
        let base = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)[0]
        return base.appendingPathComponent("mdir", isDirectory: true)
            .appendingPathComponent("mac_config.json")
    }

    static func load() -> MacAppConfig {
        guard let data = try? Data(contentsOf: configURL),
              let config = try? JSONDecoder().decode(MacAppConfig.self, from: data) else {
            return MacAppConfig()
        }
        return config
    }

    func save() {
        let url = Self.configURL
        try? FileManager.default.createDirectory(
            at: url.deletingLastPathComponent(), withIntermediateDirectories: true)
        guard let data = try? JSONEncoder().encode(self) else { return }
        try? data.write(to: url, options: .atomic)
    }
}

namespace MdirWin.Core;

/// <summary>コピー元・先フォルダの履歴とお気に入り（各最大20件）を管理する。</summary>
public sealed class FolderListManager
{
    public const int MaxItems = 20;

    public List<string> History { get; set; } = new();
    public List<string> Favorites { get; set; } = new();

    public void AddToHistory(string? path)
    {
        if (string.IsNullOrWhiteSpace(path) || !Directory.Exists(path))
            return;

        var full = Path.GetFullPath(path);
        History.RemoveAll(p => string.Equals(p, full, StringComparison.OrdinalIgnoreCase));
        History.Insert(0, full);
        while (History.Count > MaxItems)
            History.RemoveAt(History.Count - 1);
    }

    public void AddToFavorite(string? path)
    {
        if (string.IsNullOrWhiteSpace(path) || !Directory.Exists(path))
            return;

        var full = Path.GetFullPath(path);
        Favorites.RemoveAll(p => string.Equals(p, full, StringComparison.OrdinalIgnoreCase));
        Favorites.Insert(0, full);
        while (Favorites.Count > MaxItems)
            Favorites.RemoveAt(Favorites.Count - 1);
    }

    public void RemoveFromFavorites(string path)
    {
        Favorites.RemoveAll(p => string.Equals(p, path, StringComparison.OrdinalIgnoreCase));
    }

    public bool IsFavorite(string path) =>
        Favorites.Any(p => string.Equals(p, path, StringComparison.OrdinalIgnoreCase));
}

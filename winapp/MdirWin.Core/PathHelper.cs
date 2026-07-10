namespace MdirWin.Core;

/// <summary>Windows パス比較・正規化のヘルパー。</summary>
public static class PathHelper
{
    /// <summary>
    /// パスを比較用に正規化する。ドライブレターだけ（<c>D:</c>）はルート（<c>D:\</c>）に揃える。
    /// </summary>
    public static string Normalize(string path)
    {
        if (string.IsNullOrWhiteSpace(path))
            return path;

        if (path.Length == 2 && path[1] == ':')
            return path + Path.DirectorySeparatorChar;

        var full = Path.GetFullPath(path);
        if (full.Length == 2 && full[1] == ':')
            return full + Path.DirectorySeparatorChar;

        return Path.TrimEndingDirectorySeparator(full);
    }

    /// <summary>子パスが親パスの配下か（同一パスも true）。</summary>
    public static bool IsSameOrChild(string parent, string child)
    {
        var p = Normalize(parent);
        var c = Normalize(child);
        if (string.Equals(p, c, StringComparison.OrdinalIgnoreCase))
            return true;

        if (!p.EndsWith(Path.DirectorySeparatorChar))
            p += Path.DirectorySeparatorChar;

        return c.StartsWith(p, StringComparison.OrdinalIgnoreCase);
    }

    /// <summary>ドライブルートかどうか（<c>C:\</c> 形式）。</summary>
    public static bool IsDriveRoot(string path)
    {
        var n = Normalize(path);
        return n.Length == 3 && n[1] == ':' && n[2] == Path.DirectorySeparatorChar;
    }
}

namespace MdirWin.Core;

/// <summary>ミラーコピーの実行結果。</summary>
public sealed record MirrorResult(int FilesCopied, int DirectoriesCreated, IReadOnlyList<string> Errors)
{
    public bool HasErrors => Errors.Count > 0;
}

public static class FileOperations
{
    /// <summary>
    /// sourceDir 内のすべてのファイル・サブフォルダを destDir に強制上書きコピーする。
    /// 読み取り専用ファイルも属性を解除して上書きする。個別ファイルの失敗は Errors に集約する。
    /// </summary>
    public static MirrorResult MirrorCopy(string sourceDir, string destDir)
    {
        var source = Path.GetFullPath(sourceDir);
        var dest = Path.GetFullPath(destDir);

        if (!Directory.Exists(source))
            throw new DirectoryNotFoundException($"コピー元フォルダが見つかりません: {source}");
        if (PathsEqual(source, dest))
            throw new ArgumentException("コピー元とコピー先が同じフォルダです。");
        if (IsInsideDirectory(dest, source))
            throw new ArgumentException("コピー先がコピー元の中にあるためコピーできません。");

        Directory.CreateDirectory(dest);

        int filesCopied = 0;
        int dirsCreated = 0;
        var errors = new List<string>();

        foreach (var dir in Directory.EnumerateDirectories(source, "*", SearchOption.AllDirectories))
        {
            var target = Path.Combine(dest, Path.GetRelativePath(source, dir));
            try
            {
                if (!Directory.Exists(target))
                {
                    Directory.CreateDirectory(target);
                    dirsCreated++;
                }
            }
            catch (Exception ex)
            {
                errors.Add($"{target}: {ex.Message}");
            }
        }

        foreach (var file in Directory.EnumerateFiles(source, "*", SearchOption.AllDirectories))
        {
            var target = Path.Combine(dest, Path.GetRelativePath(source, file));
            try
            {
                ForceCopyFile(file, target);
                filesCopied++;
            }
            catch (Exception ex)
            {
                errors.Add($"{file}: {ex.Message}");
            }
        }

        return new MirrorResult(filesCopied, dirsCreated, errors);
    }

    /// <summary>読み取り専用・隠し属性が付いていても強制的に上書きコピーする。</summary>
    public static void ForceCopyFile(string sourceFile, string destFile)
    {
        var parent = Path.GetDirectoryName(destFile);
        if (!string.IsNullOrEmpty(parent))
            Directory.CreateDirectory(parent);

        var destInfo = new FileInfo(destFile);
        if (destInfo.Exists && (destInfo.Attributes & (FileAttributes.ReadOnly | FileAttributes.Hidden | FileAttributes.System)) != 0)
            destInfo.Attributes = FileAttributes.Normal;

        File.Copy(sourceFile, destFile, overwrite: true);
    }

    /// <summary>フォルダを再帰的にコピーする。</summary>
    public static void CopyDirectory(string sourceDir, string destDir, bool overwrite)
    {
        var source = Path.GetFullPath(sourceDir);
        var dest = Path.GetFullPath(destDir);
        if (IsInsideDirectory(dest, source) || PathsEqual(source, dest))
            throw new ArgumentException("コピー先がコピー元の中にあるためコピーできません。");

        Directory.CreateDirectory(dest);

        foreach (var dir in Directory.EnumerateDirectories(source, "*", SearchOption.AllDirectories))
            Directory.CreateDirectory(Path.Combine(dest, Path.GetRelativePath(source, dir)));

        foreach (var file in Directory.EnumerateFiles(source, "*", SearchOption.AllDirectories))
        {
            var target = Path.Combine(dest, Path.GetRelativePath(source, file));
            if (overwrite)
                ForceCopyFile(file, target);
            else
                File.Copy(file, target, overwrite: false);
        }
    }

    /// <summary>
    /// ファイルまたはフォルダを destDir 内にコピーする（貼り付け用）。
    /// 同名が存在する場合は「名前 (2)」のように一意な名前を付ける。コピー先のパスを返す。
    /// </summary>
    public static string CopyIntoDirectory(string sourcePath, string destDir)
    {
        var name = Path.GetFileName(Path.TrimEndingDirectorySeparator(sourcePath));
        var isDir = Directory.Exists(sourcePath);
        var target = GetUniquePath(Path.Combine(destDir, name), isDir);

        if (isDir)
            CopyDirectory(sourcePath, target, overwrite: false);
        else
            File.Copy(sourcePath, target);

        return target;
    }

    /// <summary>
    /// ファイルまたはフォルダを destDir 内に移動する。別ドライブ間はコピー+削除で対応する。
    /// </summary>
    public static string MoveIntoDirectory(string sourcePath, string destDir)
    {
        var name = Path.GetFileName(Path.TrimEndingDirectorySeparator(sourcePath));
        var isDir = Directory.Exists(sourcePath);
        var target = GetUniquePath(Path.Combine(destDir, name), isDir);

        try
        {
            if (isDir)
                Directory.Move(sourcePath, target);
            else
                File.Move(sourcePath, target);
        }
        catch (IOException)
        {
            // ドライブをまたぐ移動は Move が失敗するためコピー+削除にフォールバック
            if (isDir)
            {
                CopyDirectory(sourcePath, target, overwrite: false);
                Directory.Delete(sourcePath, recursive: true);
            }
            else
            {
                File.Copy(sourcePath, target);
                File.Delete(sourcePath);
            }
        }

        return target;
    }

    /// <summary>指定パスが存在する場合、「名前 (2)」形式で存在しないパスを生成する。</summary>
    public static string GetUniquePath(string desiredPath, bool isDirectory)
    {
        if (!File.Exists(desiredPath) && !Directory.Exists(desiredPath))
            return desiredPath;

        var dir = Path.GetDirectoryName(desiredPath) ?? "";
        string stem;
        string ext;
        if (isDirectory)
        {
            stem = Path.GetFileName(desiredPath);
            ext = "";
        }
        else
        {
            stem = Path.GetFileNameWithoutExtension(desiredPath);
            ext = Path.GetExtension(desiredPath);
        }

        for (var n = 2; ; n++)
        {
            var candidate = Path.Combine(dir, $"{stem} ({n}){ext}");
            if (!File.Exists(candidate) && !Directory.Exists(candidate))
                return candidate;
        }
    }

    private static bool PathsEqual(string a, string b) =>
        string.Equals(
            Path.TrimEndingDirectorySeparator(a),
            Path.TrimEndingDirectorySeparator(b),
            StringComparison.OrdinalIgnoreCase);

    /// <summary>path が directory の内側（または同一）にあるかを判定する。</summary>
    private static bool IsInsideDirectory(string path, string directory)
    {
        var p = Path.TrimEndingDirectorySeparator(Path.GetFullPath(path));
        var d = Path.TrimEndingDirectorySeparator(Path.GetFullPath(directory));
        return p.StartsWith(d + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase);
    }
}

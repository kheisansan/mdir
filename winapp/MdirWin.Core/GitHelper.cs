namespace MdirWin.Core;

public static class GitHelper
{
    /// <summary>
    /// 指定パスから上位に向かって Git リポジトリを探し、現在のブランチ名を返す。
    /// リポジトリでなければ null。detached HEAD の場合は短縮ハッシュを返す。
    /// git コマンドを起動せず .git/HEAD を直接読むため高速。
    /// </summary>
    public static string? GetBranchName(string startPath)
    {
        try
        {
            var dir = new DirectoryInfo(Path.GetFullPath(startPath));
            while (dir != null)
            {
                var gitPath = Path.Combine(dir.FullName, ".git");
                string? gitDir = null;

                if (Directory.Exists(gitPath))
                {
                    gitDir = gitPath;
                }
                else if (File.Exists(gitPath))
                {
                    // worktree / submodule: ".git" はファイルで "gitdir: <path>" を含む
                    var line = File.ReadAllText(gitPath).Trim();
                    if (line.StartsWith("gitdir:", StringComparison.Ordinal))
                    {
                        var p = line["gitdir:".Length..].Trim();
                        gitDir = Path.GetFullPath(Path.IsPathRooted(p) ? p : Path.Combine(dir.FullName, p));
                    }
                }

                if (gitDir != null)
                {
                    var headFile = Path.Combine(gitDir, "HEAD");
                    if (!File.Exists(headFile))
                        return null;

                    var head = File.ReadAllText(headFile).Trim();
                    if (head.StartsWith("ref:", StringComparison.Ordinal))
                    {
                        var reference = head[4..].Trim();
                        const string prefix = "refs/heads/";
                        return reference.StartsWith(prefix, StringComparison.Ordinal)
                            ? reference[prefix.Length..]
                            : reference;
                    }
                    return head.Length >= 7 ? head[..7] : head;
                }

                dir = dir.Parent;
            }
        }
        catch
        {
            // アクセス拒否などはリポジトリ外扱い
        }
        return null;
    }
}

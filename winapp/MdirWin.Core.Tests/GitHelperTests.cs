using MdirWin.Core;
using Xunit;

namespace MdirWin.Core.Tests;

public sealed class GitHelperTests : IDisposable
{
    private readonly string _root;

    public GitHelperTests()
    {
        _root = Path.Combine(Path.GetTempPath(), "MdirWinGitTests_" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(_root);
    }

    public void Dispose()
    {
        try
        {
            Directory.Delete(_root, recursive: true);
        }
        catch { /* テスト後始末の失敗は無視 */ }
    }

    private string MakeRepo(string relative, string headContent)
    {
        var repo = Path.Combine(_root, relative);
        var gitDir = Path.Combine(repo, ".git");
        Directory.CreateDirectory(gitDir);
        File.WriteAllText(Path.Combine(gitDir, "HEAD"), headContent);
        return repo;
    }

    [Fact]
    public void GetBranchName_ReturnsBranchFromHead()
    {
        var repo = MakeRepo("repo1", "ref: refs/heads/main\n");
        Assert.Equal("main", GitHelper.GetBranchName(repo));
    }

    [Fact]
    public void GetBranchName_SupportsSlashesInBranchName()
    {
        var repo = MakeRepo("repo2", "ref: refs/heads/feature/cool-thing\n");
        Assert.Equal("feature/cool-thing", GitHelper.GetBranchName(repo));
    }

    [Fact]
    public void GetBranchName_WalksUpToParentRepo()
    {
        var repo = MakeRepo("repo3", "ref: refs/heads/dev\n");
        var nested = Path.Combine(repo, "src", "deep");
        Directory.CreateDirectory(nested);
        Assert.Equal("dev", GitHelper.GetBranchName(nested));
    }

    [Fact]
    public void GetBranchName_DetachedHeadReturnsShortHash()
    {
        var repo = MakeRepo("repo4", "0123456789abcdef0123456789abcdef01234567\n");
        Assert.Equal("0123456", GitHelper.GetBranchName(repo));
    }

    [Fact]
    public void GetBranchName_ReturnsNullOutsideRepository()
    {
        var plain = Path.Combine(_root, "plain");
        Directory.CreateDirectory(plain);
        Assert.Null(GitHelper.GetBranchName(plain));
    }

    [Fact]
    public void GetBranchName_ResolvesGitFileForWorktree()
    {
        // 実体の git ディレクトリ
        var realGit = Path.Combine(_root, "realgit");
        Directory.CreateDirectory(realGit);
        File.WriteAllText(Path.Combine(realGit, "HEAD"), "ref: refs/heads/worktree-branch\n");

        // .git がファイルになっている worktree
        var wt = Path.Combine(_root, "worktree");
        Directory.CreateDirectory(wt);
        File.WriteAllText(Path.Combine(wt, ".git"), $"gitdir: {realGit}\n");

        Assert.Equal("worktree-branch", GitHelper.GetBranchName(wt));
    }
}

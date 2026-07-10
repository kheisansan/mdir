using MdirWin.Core;
using Xunit;

namespace MdirWin.Core.Tests;

public sealed class FileOperationsTests : IDisposable
{
    private readonly string _root;

    public FileOperationsTests()
    {
        _root = Path.Combine(Path.GetTempPath(), "MdirWinTests_" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(_root);
    }

    public void Dispose()
    {
        try
        {
            ClearReadOnly(_root);
            Directory.Delete(_root, recursive: true);
        }
        catch { /* テスト後始末の失敗は無視 */ }
    }

    private static void ClearReadOnly(string dir)
    {
        foreach (var f in Directory.EnumerateFiles(dir, "*", SearchOption.AllDirectories))
            File.SetAttributes(f, FileAttributes.Normal);
    }

    private string MakeDir(params string[] parts)
    {
        var p = Path.Combine(new[] { _root }.Concat(parts).ToArray());
        Directory.CreateDirectory(p);
        return p;
    }

    private static void WriteFile(string dir, string name, string content)
    {
        Directory.CreateDirectory(dir);
        File.WriteAllText(Path.Combine(dir, name), content);
    }

    [Fact]
    public void MirrorCopy_CopiesAllFilesAndSubdirectories()
    {
        var src = MakeDir("src");
        var dst = MakeDir("dst");
        WriteFile(src, "a.txt", "A");
        WriteFile(Path.Combine(src, "sub"), "b.txt", "B");
        WriteFile(Path.Combine(src, "sub", "deep"), "c.txt", "C");

        var result = FileOperations.MirrorCopy(src, dst);

        Assert.False(result.HasErrors);
        Assert.Equal(3, result.FilesCopied);
        Assert.Equal("A", File.ReadAllText(Path.Combine(dst, "a.txt")));
        Assert.Equal("B", File.ReadAllText(Path.Combine(dst, "sub", "b.txt")));
        Assert.Equal("C", File.ReadAllText(Path.Combine(dst, "sub", "deep", "c.txt")));
    }

    [Fact]
    public void MirrorCopy_OverwritesExistingFiles()
    {
        var src = MakeDir("src");
        var dst = MakeDir("dst");
        WriteFile(src, "a.txt", "new");
        WriteFile(dst, "a.txt", "old");

        FileOperations.MirrorCopy(src, dst);

        Assert.Equal("new", File.ReadAllText(Path.Combine(dst, "a.txt")));
    }

    [Fact]
    public void MirrorCopy_OverwritesReadOnlyFiles()
    {
        var src = MakeDir("src");
        var dst = MakeDir("dst");
        WriteFile(src, "a.txt", "new");
        WriteFile(dst, "a.txt", "old");
        File.SetAttributes(Path.Combine(dst, "a.txt"), FileAttributes.ReadOnly);

        var result = FileOperations.MirrorCopy(src, dst);

        Assert.False(result.HasErrors);
        Assert.Equal("new", File.ReadAllText(Path.Combine(dst, "a.txt")));
    }

    [Fact]
    public void MirrorCopy_CreatesDestinationIfMissing()
    {
        var src = MakeDir("src");
        WriteFile(src, "a.txt", "A");
        var dst = Path.Combine(_root, "not_yet");

        var result = FileOperations.MirrorCopy(src, dst);

        Assert.Equal(1, result.FilesCopied);
        Assert.True(File.Exists(Path.Combine(dst, "a.txt")));
    }

    [Fact]
    public void MirrorCopy_ThrowsWhenSourceMissing()
    {
        Assert.Throws<DirectoryNotFoundException>(
            () => FileOperations.MirrorCopy(Path.Combine(_root, "nope"), MakeDir("dst")));
    }

    [Fact]
    public void MirrorCopy_ThrowsWhenSourceEqualsDestination()
    {
        var src = MakeDir("src");
        Assert.Throws<ArgumentException>(() => FileOperations.MirrorCopy(src, src));
    }

    [Fact]
    public void MirrorCopy_ThrowsWhenDestinationInsideSource()
    {
        var src = MakeDir("src");
        var dst = MakeDir("src", "inner");
        Assert.Throws<ArgumentException>(() => FileOperations.MirrorCopy(src, dst));
    }

    [Fact]
    public void CopyIntoDirectory_CopiesFile()
    {
        var src = MakeDir("src");
        var dst = MakeDir("dst");
        WriteFile(src, "a.txt", "A");

        var target = FileOperations.CopyIntoDirectory(Path.Combine(src, "a.txt"), dst);

        Assert.Equal(Path.Combine(dst, "a.txt"), target);
        Assert.Equal("A", File.ReadAllText(target));
    }

    [Fact]
    public void CopyIntoDirectory_CopiesDirectoryRecursively()
    {
        var src = MakeDir("src", "folder");
        WriteFile(src, "a.txt", "A");
        WriteFile(Path.Combine(src, "sub"), "b.txt", "B");
        var dst = MakeDir("dst");

        var target = FileOperations.CopyIntoDirectory(src, dst);

        Assert.Equal(Path.Combine(dst, "folder"), target);
        Assert.Equal("B", File.ReadAllText(Path.Combine(target, "sub", "b.txt")));
    }

    [Fact]
    public void CopyIntoDirectory_RenamesOnNameConflict()
    {
        var src = MakeDir("src");
        var dst = MakeDir("dst");
        WriteFile(src, "a.txt", "new");
        WriteFile(dst, "a.txt", "old");

        var target = FileOperations.CopyIntoDirectory(Path.Combine(src, "a.txt"), dst);

        Assert.Equal(Path.Combine(dst, "a (2).txt"), target);
        Assert.Equal("old", File.ReadAllText(Path.Combine(dst, "a.txt")));
        Assert.Equal("new", File.ReadAllText(target));
    }

    [Fact]
    public void MoveIntoDirectory_MovesFile()
    {
        var src = MakeDir("src");
        var dst = MakeDir("dst");
        WriteFile(src, "a.txt", "A");

        var target = FileOperations.MoveIntoDirectory(Path.Combine(src, "a.txt"), dst);

        Assert.False(File.Exists(Path.Combine(src, "a.txt")));
        Assert.Equal("A", File.ReadAllText(target));
    }

    [Fact]
    public void GetUniquePath_ReturnsSamePathWhenFree()
    {
        var p = Path.Combine(_root, "free.txt");
        Assert.Equal(p, FileOperations.GetUniquePath(p, isDirectory: false));
    }

    [Fact]
    public void GetUniquePath_IncrementsUntilFree()
    {
        WriteFile(_root, "a.txt", "1");
        WriteFile(_root, "a (2).txt", "2");

        var p = FileOperations.GetUniquePath(Path.Combine(_root, "a.txt"), isDirectory: false);

        Assert.Equal(Path.Combine(_root, "a (3).txt"), p);
    }

    [Fact]
    public void GetUniquePath_DirectoryKeepsDotInName()
    {
        MakeDir("my.folder");

        var p = FileOperations.GetUniquePath(Path.Combine(_root, "my.folder"), isDirectory: true);

        Assert.Equal(Path.Combine(_root, "my.folder (2)"), p);
    }
}

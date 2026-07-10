using MdirWin.Core;
using Xunit;

namespace MdirWin.Core.Tests;

public sealed class FolderListManagerTests
{
    [Fact]
    public void AddToHistory_KeepsMostRecentFirst_AndCapsAtMax()
    {
        var mgr = new FolderListManager();
        for (var i = 0; i < FolderListManager.MaxItems + 3; i++)
        {
            var dir = Path.Combine(Path.GetTempPath(), "MdirWinHist_" + Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(dir);
            mgr.AddToHistory(dir);
        }

        Assert.Equal(FolderListManager.MaxItems, mgr.History.Count);
    }

    [Fact]
    public void AddToFavorite_DoesNotDuplicate()
    {
        var dir = Path.Combine(Path.GetTempPath(), "MdirWinFav_" + Guid.NewGuid().ToString("N"));
        Directory.CreateDirectory(dir);

        var mgr = new FolderListManager();
        mgr.AddToFavorite(dir);
        mgr.AddToFavorite(dir);

        Assert.Single(mgr.Favorites);
    }
}

using MdirWin.Core;
using Xunit;

namespace MdirWin.Core.Tests;

public sealed class PathHelperTests
{
    [Theory]
    [InlineData(@"C:\", @"C:\")]
    [InlineData(@"C:", @"C:\")]
    [InlineData(@"D:\foo", @"D:\foo")]
    public void Normalize_HandlesDriveRoots(string input, string expected)
    {
        Assert.Equal(expected, PathHelper.Normalize(input));
    }

    [Fact]
    public void IsSameOrChild_MatchesDescendants()
    {
        Assert.True(PathHelper.IsSameOrChild(@"C:\", @"C:\Users"));
        Assert.True(PathHelper.IsSameOrChild(@"D:\", @"D:\"));
        Assert.False(PathHelper.IsSameOrChild(@"C:\", @"D:\Users"));
    }

    [Fact]
    public void IsDriveRoot_DetectsRoot()
    {
        Assert.True(PathHelper.IsDriveRoot(@"C:\"));
        Assert.True(PathHelper.IsDriveRoot(@"D:"));
        Assert.False(PathHelper.IsDriveRoot(@"C:\Users"));
    }
}

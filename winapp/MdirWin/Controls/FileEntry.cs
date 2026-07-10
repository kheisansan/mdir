using System.ComponentModel;
using System.Globalization;
using System.IO;

namespace MdirWin.Controls;

/// <summary>ファイルペインの1行分の表示モデル。</summary>
public sealed class FileEntry : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler? PropertyChanged;

    public required string FullPath { get; init; }
    public required string Name { get; init; }
    public required bool IsDirectory { get; init; }
    public bool IsParentLink { get; init; }
    public bool IsHidden { get; init; }
    public long Size { get; init; }
    public DateTime Modified { get; init; }

    private bool _isMarked;

    /// <summary>Space / Ins によるマーク状態。</summary>
    public bool IsMarked
    {
        get => _isMarked;
        set
        {
            if (_isMarked == value)
                return;
            _isMarked = value;
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(IsMarked)));
            PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(MarkText)));
        }
    }

    public string MarkText => IsMarked ? "✓" : "";
    public string Icon => IsParentLink ? "⬆" : IsDirectory ? "📁" : "📄";
    public string SizeText => IsDirectory ? "<DIR>" : FormatSize(Size);
    public string ModifiedText => IsParentLink ? "" : Modified.ToString("yyyy-MM-dd HH:mm", CultureInfo.InvariantCulture);
    public string Extension => IsDirectory ? "" : Path.GetExtension(Name);

    public static string FormatSize(long bytes)
    {
        if (bytes < 1024) return bytes.ToString("N0");
        string[] units = ["KB", "MB", "GB", "TB"];
        double v = bytes;
        var i = -1;
        while (v >= 1024 && i < units.Length - 1)
        {
            v /= 1024;
            i++;
        }
        return $"{v:0.#} {units[i]}";
    }

    public static FileEntry FromFileSystemInfo(FileSystemInfo info)
    {
        var isDir = info is DirectoryInfo;
        return new FileEntry
        {
            FullPath = info.FullName,
            Name = info.Name,
            IsDirectory = isDir,
            IsHidden = (info.Attributes & FileAttributes.Hidden) != 0,
            Size = isDir ? 0 : ((FileInfo)info).Length,
            Modified = info.LastWriteTime,
        };
    }
}

using System.IO;
using System.Text.Json;

namespace MdirWin;

/// <summary>アプリ設定（%APPDATA%\MdirWin\config.json に保存）。</summary>
public sealed class AppConfig
{
    public const double DefaultFontSize = 13;

    public string? SourceFolder { get; set; }
    public string? DestFolder { get; set; }
    public string? LeftPanePath { get; set; }
    public string? RightPanePath { get; set; }
    public double FontSize { get; set; } = DefaultFontSize;
    public List<string> SourceHistory { get; set; } = new();
    public List<string> DestHistory { get; set; } = new();
    public List<string> SourceFavorites { get; set; } = new();
    public List<string> DestFavorites { get; set; } = new();

    private static string ConfigPath => Path.Combine(
        Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
        "MdirWin", "config.json");

    public static AppConfig Load()
    {
        try
        {
            if (File.Exists(ConfigPath))
                return JsonSerializer.Deserialize<AppConfig>(File.ReadAllText(ConfigPath)) ?? new AppConfig();
        }
        catch { /* 壊れた設定は初期化 */ }
        return new AppConfig();
    }

    public void Save()
    {
        var dir = Path.GetDirectoryName(ConfigPath)!;
        Directory.CreateDirectory(dir);
        File.WriteAllText(ConfigPath, JsonSerializer.Serialize(this, new JsonSerializerOptions { WriteIndented = true }));
    }
}

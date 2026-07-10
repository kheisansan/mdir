using System.Diagnostics;

namespace MdirWin;

internal static class ProcessRunner
{
    /// <summary>外部コマンドを実行し、終了コードと標準出力+標準エラーを返す。</summary>
    public static async Task<(int ExitCode, string Output)> RunAsync(string fileName, string arguments, string workingDirectory)
    {
        var psi = new ProcessStartInfo(fileName, arguments)
        {
            WorkingDirectory = workingDirectory,
            UseShellExecute = false,
            RedirectStandardOutput = true,
            RedirectStandardError = true,
            CreateNoWindow = true,
        };

        using var process = Process.Start(psi)
            ?? throw new InvalidOperationException($"プロセスを起動できませんでした: {fileName}");

        var stdout = process.StandardOutput.ReadToEndAsync();
        var stderr = process.StandardError.ReadToEndAsync();
        await process.WaitForExitAsync();

        var output = ((await stdout) + Environment.NewLine + (await stderr)).Trim();
        return (process.ExitCode, output);
    }
}

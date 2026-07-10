using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;

namespace MdirWin;

/// <summary>
/// RegisterHotKey によるグローバルショートカット管理。
/// 他のアプリにフォーカスがあっても発火する。
/// </summary>
public sealed class GlobalHotkeys : IDisposable
{
    private const int WmHotkey = 0x0312;

    private const uint ModShift = 0x0004;
    private const uint ModControl = 0x0002;
    private const uint ModWin = 0x0008;
    private const uint ModNoRepeat = 0x4000;

    private const uint VkOem4 = 0xDB; // [
    private const uint VkOem6 = 0xDD; // ]

    private const int IdMirrorForward = 0xA001;  // コピー元 → コピー先
    private const int IdMirrorBackward = 0xA002; // コピー先 → コピー元

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool RegisterHotKey(IntPtr hWnd, int id, uint fsModifiers, uint vk);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern bool UnregisterHotKey(IntPtr hWnd, int id);

    private readonly IntPtr _hwnd;
    private readonly HwndSource _source;
    private readonly Action _onMirrorForward;
    private readonly Action _onMirrorBackward;
    private bool _disposed;

    public bool ForwardRegistered { get; }
    public bool BackwardRegistered { get; }

    public GlobalHotkeys(Window window, Action onMirrorForward, Action onMirrorBackward)
    {
        _onMirrorForward = onMirrorForward;
        _onMirrorBackward = onMirrorBackward;

        var helper = new WindowInteropHelper(window);
        _hwnd = helper.EnsureHandle();
        _source = HwndSource.FromHwnd(_hwnd)!;
        _source.AddHook(WndProc);

        const uint mods = ModWin | ModControl | ModShift | ModNoRepeat;
        ForwardRegistered = RegisterHotKey(_hwnd, IdMirrorForward, mods, VkOem4);
        BackwardRegistered = RegisterHotKey(_hwnd, IdMirrorBackward, mods, VkOem6);
    }

    private IntPtr WndProc(IntPtr hwnd, int msg, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        if (msg == WmHotkey)
        {
            switch (wParam.ToInt32())
            {
                case IdMirrorForward:
                    _onMirrorForward();
                    handled = true;
                    break;
                case IdMirrorBackward:
                    _onMirrorBackward();
                    handled = true;
                    break;
            }
        }
        return IntPtr.Zero;
    }

    public void Dispose()
    {
        if (_disposed)
            return;
        _disposed = true;
        UnregisterHotKey(_hwnd, IdMirrorForward);
        UnregisterHotKey(_hwnd, IdMirrorBackward);
        _source.RemoveHook(WndProc);
    }
}

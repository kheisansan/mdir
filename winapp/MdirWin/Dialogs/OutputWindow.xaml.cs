using System.Windows;

namespace MdirWin.Dialogs;

/// <summary>コマンド出力やヘルプなどの長文テキスト表示ウィンドウ。</summary>
public partial class OutputWindow : Window
{
    public OutputWindow(string title, string text)
    {
        InitializeComponent();
        Title = title;
        OutputBox.Text = text;
    }

    public static void ShowText(Window owner, string title, string text)
    {
        var win = new OutputWindow(title, text) { Owner = owner };
        win.Show();
    }
}

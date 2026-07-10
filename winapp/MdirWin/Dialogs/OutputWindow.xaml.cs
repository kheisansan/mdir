using System.Windows;
using System.Windows.Input;

namespace MdirWin.Dialogs;

/// <summary>コマンド出力やヘルプなどの長文テキスト表示ウィンドウ。</summary>
public partial class OutputWindow : Window
{
    public OutputWindow(string title, string text)
    {
        InitializeComponent();
        Title = title;
        OutputBox.Text = text;
        Loaded += (_, _) => CloseButton.Focus();
    }

    public static void ShowText(Window owner, string title, string text)
    {
        var win = new OutputWindow(title, text) { Owner = owner };
        win.Show();
    }

    private void CloseButton_Click(object sender, RoutedEventArgs e) => Close();

    private void Window_PreviewKeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Escape)
        {
            Close();
            e.Handled = true;
        }
    }
}

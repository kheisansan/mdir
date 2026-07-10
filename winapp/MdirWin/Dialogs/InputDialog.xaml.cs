using System.Windows;
using System.Windows.Input;

namespace MdirWin.Dialogs;

public partial class InputDialog : Window
{
    public string Value => ValueBox.Text;

    public InputDialog(string title, string prompt, string initialValue = "")
    {
        InitializeComponent();
        Title = title;
        PromptText.Text = prompt;
        ValueBox.Text = initialValue;
        Loaded += (_, _) =>
        {
            ValueBox.Focus();
            ValueBox.SelectAll();
        };
    }

    /// <summary>入力値を取得する。キャンセル時は null。</summary>
    public static string? Show(Window owner, string title, string prompt, string initialValue = "")
    {
        var dlg = new InputDialog(title, prompt, initialValue) { Owner = owner };
        return dlg.ShowDialog() == true ? dlg.Value : null;
    }

    private void Ok_Click(object sender, RoutedEventArgs e)
    {
        DialogResult = true;
    }

    private void ValueBox_KeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter)
            DialogResult = true;
    }
}

using System.Windows;
using System.Windows.Media;
using System.Windows.Threading;

namespace MdirWin.Dialogs;

/// <summary>画面右下に数秒表示して自動で消える通知ウィンドウ。</summary>
public partial class ToastWindow : Window
{
    private ToastWindow(string title, string body, bool isError)
    {
        InitializeComponent();
        TitleText.Text = title;
        BodyText.Text = body;
        if (isError)
            ToastBorder.Background = new SolidColorBrush(Color.FromArgb(0xEE, 0x8B, 0x1E, 0x1E));
    }

    public static void Show(string title, string body, bool isError = false, double seconds = 3.0)
    {
        var toast = new ToastWindow(title, body, isError);
        toast.Loaded += (_, _) =>
        {
            var area = SystemParameters.WorkArea;
            toast.Left = area.Right - toast.ActualWidth - 16;
            toast.Top = area.Bottom - toast.ActualHeight - 16;
        };
        toast.Show();

        var timer = new DispatcherTimer { Interval = TimeSpan.FromSeconds(seconds) };
        timer.Tick += (_, _) =>
        {
            timer.Stop();
            toast.Close();
        };
        timer.Start();
    }
}

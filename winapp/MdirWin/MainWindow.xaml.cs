using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using MdirWin.Controls;
using MdirWin.Dialogs;

namespace MdirWin;

public partial class MainWindow : Window
{
    private const string HelpText = """
        Mdir for Windows — キーバインド

        ── ナビゲーション ──────────────────────
          h / ← / BS      親フォルダへ移動
          j / ↓           カーソルを下に移動
          k / ↑           カーソルを上に移動
          l / → / Enter   フォルダに入る / ファイルを開く
          g / Home        先頭へジャンプ
          G / End         末尾へジャンプ
          PgUp / PgDn     ページ単位で移動
          ~               ホームフォルダへ移動
          Tab             アクティブペインを切り替え

        ── ファイル操作 ────────────────────────
          c / F5          反対ペインへコピー
          m / F6          反対ペインへ移動
          d / F8 / Del    削除（ゴミ箱へ移動）
          r / F2          名前の変更
          n / F7          新規フォルダ作成
          t / F4          新規ファイル作成
          Space / Ins     マーク / マーク解除
          a               全マーク / 全解除
          s               ソートメニュー
          .               隠しファイル表示切り替え
          i               ファイル情報
          y               Git Pull（リポジトリ内のみ）
          Ctrl+C/X/V      クリップボード（エクスプローラー互換）
          Ctrl+R          最新の情報に更新

        ── コマンドモード（:）─────────────────
          :help           このヘルプを表示
          :q / :quit      終了
          :cd <パス>      フォルダ移動
          :sort <基準>    ソート（name / size / date / ext）
          :find <文字列>  ファイル名検索（o: 前へ / p: 次へ）
          :mkdir <名前>   フォルダ作成
          :touch <名前>   ファイル作成
          :!<コマンド>    シェルコマンド実行

        ── グローバルショートカット（他アプリ使用中も有効）──
          Win+Ctrl+Shift+[   コピー元 → コピー先へ強制上書きコピー
          Win+Ctrl+Shift+]   コピー先 → コピー元へ強制上書きコピー
        """;

    private readonly AppConfig _config;
    private GlobalHotkeys? _hotkeys;
    private FilePaneControl _activePane;
    private bool _mirrorRunning;

    public MainWindow()
    {
        InitializeComponent();
        _config = AppConfig.Load();
        _activePane = LeftPane;

        LeftPane.GetOtherPane = () => RightPane;
        RightPane.GetOtherPane = () => LeftPane;
        LeftPane.ActivatedByUser += (_, _) => SetActivePane(LeftPane);
        RightPane.ActivatedByUser += (_, _) => SetActivePane(RightPane);

        // ツリーとペインの対応: 左ツリー ⇔ 左ペイン、右ツリー ⇔ 右ペイン
        LeftTree.FolderSelected += (_, path) => LeftPane.NavigateTo(path);
        RightTree.FolderSelected += (_, path) => RightPane.NavigateTo(path);
        LeftPane.PathChanged += (_, path) =>
        {
            LeftTree.ExpandToPath(path, select: true);
            UpdateStatusBar();
        };
        RightPane.PathChanged += (_, path) =>
        {
            RightTree.ExpandToPath(path, select: true);
            UpdateStatusBar();
        };
        LeftTree.TreeChanged += (_, _) => OnTreeChanged();
        RightTree.TreeChanged += (_, _) => OnTreeChanged();

        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        LeftPane.NavigateTo(Directory.Exists(_config.LeftPanePath ?? "") ? _config.LeftPanePath! : home);
        RightPane.NavigateTo(Directory.Exists(_config.RightPanePath ?? "") ? _config.RightPanePath! : home);
        SetActivePane(LeftPane);

        SourceFolderBox.Text = _config.SourceFolder ?? "";
        DestFolderBox.Text = _config.DestFolder ?? "";

        Loaded += MainWindow_Loaded;
        Closing += MainWindow_Closing;

        // キー操作はアプリ活性中のみ有効（ウィンドウ内のルーティング。グローバルフックではない）
        PreviewKeyDown += Window_PreviewKeyDown;
        PreviewTextInput += Window_PreviewTextInput;
    }

    private void MainWindow_Loaded(object sender, RoutedEventArgs e)
    {
        _hotkeys = new GlobalHotkeys(
            this,
            onMirrorForward: () => RunMirror(forward: true),
            onMirrorBackward: () => RunMirror(forward: false));

        if (!_hotkeys.ForwardRegistered || !_hotkeys.BackwardRegistered)
        {
            ToastWindow.Show("ショートカット登録失敗",
                "グローバルショートカットの一部を登録できませんでした。他のアプリと競合している可能性があります。",
                isError: true, seconds: 5);
        }

        LeftPane.FocusList();
    }

    private void MainWindow_Closing(object? sender, System.ComponentModel.CancelEventArgs e)
    {
        _hotkeys?.Dispose();
        _config.LeftPanePath = LeftPane.CurrentPath;
        _config.RightPanePath = RightPane.CurrentPath;
        _config.SourceFolder = SourceFolderBox.Text;
        _config.DestFolder = DestFolderBox.Text;
        _config.Save();
    }

    private void SetActivePane(FilePaneControl pane)
    {
        _activePane = pane;
        LeftPane.SetActive(pane == LeftPane);
        RightPane.SetActive(pane == RightPane);
        UpdateStatusBar();
    }

    private void SwitchActivePane()
    {
        SetActivePane(_activePane == LeftPane ? RightPane : LeftPane);
        _activePane.FocusList();
    }

    private void UpdateStatusBar()
    {
        StatusPathText.Text = $"{(_activePane == LeftPane ? "[左]" : "[右]")} {_activePane.CurrentPath}";
        StatusBranchText.Text = _activePane.CurrentBranch != null ? $"⎇ {_activePane.CurrentBranch}" : "";
    }

    public void RefreshTrees()
    {
        LeftTree.ReloadPreservingExpansion();
        RightTree.ReloadPreservingExpansion();
    }

    private void OnTreeChanged()
    {
        LeftPane.Refresh();
        RightPane.Refresh();
        RefreshTrees();
    }

    // ---- キーボード操作（アプリ活性中のみ） ----

    private static bool IsTextInputFocused() =>
        Keyboard.FocusedElement is TextBoxBase or PasswordBox;

    private void Window_PreviewKeyDown(object sender, KeyEventArgs e)
    {
        if (IsTextInputFocused())
            return;

        var pane = _activePane;
        var ctrl = Keyboard.Modifiers.HasFlag(ModifierKeys.Control);

        switch (e.Key)
        {
            case Key.Down:
                pane.MoveCursor(1);
                break;
            case Key.Up:
                pane.MoveCursor(-1);
                break;
            case Key.Left:
            case Key.Back:
                pane.GoParent();
                break;
            case Key.Right:
            case Key.Enter:
                pane.OpenSelected();
                break;
            case Key.Home:
                pane.CursorToStart();
                break;
            case Key.End:
                pane.CursorToEnd();
                break;
            case Key.PageUp:
                pane.PageMove(-1);
                break;
            case Key.PageDown:
                pane.PageMove(1);
                break;
            case Key.Tab:
                SwitchActivePane();
                break;
            case Key.Space:
            case Key.Insert:
                pane.ToggleMarkAtCursor();
                break;
            case Key.Delete:
            case Key.F8:
                pane.DeleteSelected();
                break;
            case Key.F2:
                pane.RenameSelected();
                break;
            case Key.F4:
                pane.CreateNewFile();
                break;
            case Key.F5:
                pane.CopySelectedToOtherPane();
                break;
            case Key.F6:
                pane.MoveSelectedToOtherPane();
                break;
            case Key.F7:
                pane.CreateNewFolder();
                break;
            case Key.C when ctrl:
                pane.CopySelectionToClipboard(cut: false);
                break;
            case Key.X when ctrl:
                pane.CopySelectionToClipboard(cut: true);
                break;
            case Key.V when ctrl:
                pane.PasteFromClipboard();
                break;
            case Key.R when ctrl:
                pane.Refresh();
                break;
            default:
                return;
        }
        e.Handled = true;
    }

    private void Window_PreviewTextInput(object sender, TextCompositionEventArgs e)
    {
        if (IsTextInputFocused())
            return;

        var pane = _activePane;
        switch (e.Text)
        {
            case ":":
                ShowCommandLine();
                break;
            case "h":
                pane.GoParent();
                break;
            case "j":
                pane.MoveCursor(1);
                break;
            case "k":
                pane.MoveCursor(-1);
                break;
            case "l":
                pane.OpenSelected();
                break;
            case "g":
                pane.CursorToStart();
                break;
            case "G":
                pane.CursorToEnd();
                break;
            case "~":
                pane.GoHome();
                break;
            case "c":
                pane.CopySelectedToOtherPane();
                break;
            case "m":
                pane.MoveSelectedToOtherPane();
                break;
            case "d":
                pane.DeleteSelected();
                break;
            case "r":
                pane.RenameSelected();
                break;
            case "n":
                pane.CreateNewFolder();
                break;
            case "t":
                pane.CreateNewFile();
                break;
            case "a":
                pane.ToggleMarkAll();
                break;
            case "s":
                pane.ShowSortMenu();
                break;
            case ".":
                pane.ToggleHidden();
                break;
            case "i":
                pane.ShowFileInfo();
                break;
            case "y":
                pane.GitPull();
                break;
            case "o":
                pane.FindPrev();
                break;
            case "p":
                pane.FindNext();
                break;
            case " ":
                // Space は KeyDown 側で処理済み
                break;
            default:
                return;
        }
        e.Handled = true;
    }

    // ---- コマンドモード ----

    private void ShowCommandLine()
    {
        StatusBarPanel.Visibility = Visibility.Collapsed;
        CommandBox.Visibility = Visibility.Visible;
        CommandBox.Text = ":";
        CommandBox.Focus();
        CommandBox.CaretIndex = CommandBox.Text.Length;
    }

    private void HideCommandLine()
    {
        CommandBox.Visibility = Visibility.Collapsed;
        StatusBarPanel.Visibility = Visibility.Visible;
        _activePane.FocusList();
    }

    private void CommandBox_PreviewKeyDown(object sender, KeyEventArgs e)
    {
        switch (e.Key)
        {
            case Key.Enter:
                var command = CommandBox.Text;
                HideCommandLine();
                ExecuteCommand(command);
                e.Handled = true;
                break;
            case Key.Escape:
                HideCommandLine();
                e.Handled = true;
                break;
        }
    }

    private void ExecuteCommand(string raw)
    {
        var text = raw.TrimStart(':').Trim();
        if (text.Length == 0)
            return;

        if (text.StartsWith('!'))
        {
            RunShellCommand(text[1..].Trim());
            return;
        }

        var parts = text.Split(' ', 2, StringSplitOptions.TrimEntries);
        var arg = parts.Length > 1 ? parts[1] : "";

        switch (parts[0].ToLowerInvariant())
        {
            case "q" or "quit":
                Close();
                break;
            case "help":
                OutputWindow.ShowText(this, "ヘルプ — Mdir for Windows", HelpText);
                break;
            case "cd" when arg.Length > 0:
                var target = arg == "~" || arg.StartsWith("~/") || arg.StartsWith("~\\")
                    ? Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), arg.TrimStart('~', '/', '\\'))
                    : arg;
                _activePane.NavigateTo(Path.IsPathRooted(target) ? target : Path.Combine(_activePane.CurrentPath, target));
                break;
            case "sort" when arg.Length > 0:
                _activePane.SetSort(arg.ToLowerInvariant() switch
                {
                    "size" => SortField.Size,
                    "date" => SortField.Date,
                    "ext" => SortField.Ext,
                    _ => SortField.Name,
                });
                break;
            case "find" when arg.Length > 0:
                _activePane.Find(arg);
                break;
            case "mkdir" when arg.Length > 0:
                _activePane.CreateNewFolder(arg);
                break;
            case "touch" when arg.Length > 0:
                _activePane.CreateNewFile(arg);
                break;
            default:
                ToastWindow.Show("コマンド", $"不明なコマンドです: {text}（:help でヘルプ）", isError: true, seconds: 3);
                break;
        }
    }

    private async void RunShellCommand(string command)
    {
        if (command.Length == 0)
            return;

        try
        {
            var (code, output) = await ProcessRunner.RunAsync("cmd.exe", "/c " + command, _activePane.CurrentPath);
            OutputWindow.ShowText(this, $":!{command}", output + $"\n\n[exit code: {code}]");
            LeftPane.Refresh();
            RightPane.Refresh();
        }
        catch (Exception ex)
        {
            ToastWindow.Show("コマンド実行エラー", ex.Message, isError: true, seconds: 5);
        }
    }

    // ---- コピー元 / コピー先 ----

    private void BrowseSource_Click(object sender, RoutedEventArgs e)
    {
        if (BrowseFolder("コピー元のフォルダを選択", SourceFolderBox.Text) is { } path)
        {
            SourceFolderBox.Text = path;
            _config.SourceFolder = path;
            _config.Save();
        }
    }

    private void BrowseDest_Click(object sender, RoutedEventArgs e)
    {
        if (BrowseFolder("コピー先のフォルダを選択", DestFolderBox.Text) is { } path)
        {
            DestFolderBox.Text = path;
            _config.DestFolder = path;
            _config.Save();
        }
    }

    private string? BrowseFolder(string description, string? initial)
    {
        using var dialog = new System.Windows.Forms.FolderBrowserDialog
        {
            Description = description,
            UseDescriptionForTitle = true,
            ShowNewFolderButton = true,
        };
        if (!string.IsNullOrEmpty(initial) && Directory.Exists(initial))
            dialog.InitialDirectory = initial;

        return dialog.ShowDialog() == System.Windows.Forms.DialogResult.OK
            ? dialog.SelectedPath
            : null;
    }

    // ---- グローバルショートカットによるミラーコピー ----

    private async void RunMirror(bool forward)
    {
        if (_mirrorRunning)
        {
            ToastWindow.Show("実行中", "前回のコピーがまだ実行中です。", isError: true);
            return;
        }

        var source = forward ? SourceFolderBox.Text : DestFolderBox.Text;
        var dest = forward ? DestFolderBox.Text : SourceFolderBox.Text;
        var label = forward ? "コピー元 → コピー先" : "コピー先 → コピー元";

        if (string.IsNullOrWhiteSpace(source) || string.IsNullOrWhiteSpace(dest))
        {
            ToastWindow.Show("設定不足", "サイドバーでコピー元とコピー先のフォルダを設定してください。", isError: true, seconds: 5);
            return;
        }

        _mirrorRunning = true;
        try
        {
            var result = await Task.Run(() => Core.FileOperations.MirrorCopy(source, dest));
            if (result.HasErrors)
            {
                ToastWindow.Show($"{label}: 一部失敗",
                    $"{result.FilesCopied} 件コピー / {result.Errors.Count} 件失敗\n" +
                    string.Join("\n", result.Errors.Take(3)),
                    isError: true, seconds: 6);
            }
            else
            {
                ToastWindow.Show(label,
                    $"{result.FilesCopied} ファイルを上書きコピーしました\n{source}\n→ {dest}");
            }

            LeftPane.Refresh();
            RightPane.Refresh();
        }
        catch (Exception ex)
        {
            ToastWindow.Show($"{label}: エラー", ex.Message, isError: true, seconds: 6);
        }
        finally
        {
            _mirrorRunning = false;
        }
    }
}

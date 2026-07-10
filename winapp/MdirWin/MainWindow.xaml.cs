using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using MdirWin.Controls;
using MdirWin.Core;
using MdirWin.Dialogs;

namespace MdirWin;

public partial class MainWindow : Window
{
    private const double MinFontSize = AppConfig.DefaultFontSize;
    private const double MaxFontSize = 24;
    private const double FontStep = 1;

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
          1               現在のフォルダ名をクリップボードへコピー
          2               現在のフォルダのフルパスをクリップボードへコピー
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

        ── 表示 ───────────────────────────────
          Ctrl++          フォントを拡大
          Ctrl+-          フォントを縮小
          Ctrl+0          フォントをリセット

        ── サイドバー ─────────────────────────
          コピー元・コピー先フォルダは履歴（各20件）と
          お気に入り（各20件）を管理できます。
          リストを右クリックしてお気に入りに登録できます。

        ── グローバルショートカット（他アプリ使用中も有効）──
          Win+Ctrl+Shift+[   コピー元 → コピー先へ強制上書きコピー
          Win+Ctrl+Shift+]   コピー先 → コピー元へ強制上書きコピー

        ── メニューバー ───────────────────────
          上のメニューからも主要な操作を実行できます。
        """;

    private readonly AppConfig _config;
    private readonly FolderListManager _sourceFolders = new();
    private readonly FolderListManager _destFolders = new();
    private GlobalHotkeys? _hotkeys;
    private FilePaneControl _activePane;
    private bool _mirrorRunning;
    private bool _suppressFolderListSelection;
    private double _fontSize;

    public MainWindow()
    {
        InitializeComponent();
        _config = AppConfig.Load();
        _activePane = LeftPane;
        _fontSize = _config.FontSize > 0 ? _config.FontSize : AppConfig.DefaultFontSize;

        LoadFolderListsFromConfig();

        LeftPane.GetOtherPane = () => RightPane;
        RightPane.GetOtherPane = () => LeftPane;
        LeftPane.ActivatedByUser += (_, _) => SetActivePane(LeftPane);
        RightPane.ActivatedByUser += (_, _) => SetActivePane(RightPane);

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

        if (!string.IsNullOrWhiteSpace(_config.SourceFolder))
            SetSourceFolder(_config.SourceFolder, addToHistory: false);
        if (!string.IsNullOrWhiteSpace(_config.DestFolder))
            SetDestFolder(_config.DestFolder, addToHistory: false);

        ApplyFontSize(_fontSize);

        Loaded += MainWindow_Loaded;
        Closing += MainWindow_Closing;

        PreviewKeyDown += Window_PreviewKeyDown;
        PreviewTextInput += Window_PreviewTextInput;
    }

    private void LoadFolderListsFromConfig()
    {
        _sourceFolders.History = _config.SourceHistory.Take(FolderListManager.MaxItems).ToList();
        _sourceFolders.Favorites = _config.SourceFavorites.Take(FolderListManager.MaxItems).ToList();
        _destFolders.History = _config.DestHistory.Take(FolderListManager.MaxItems).ToList();
        _destFolders.Favorites = _config.DestFavorites.Take(FolderListManager.MaxItems).ToList();
        RefreshFolderListsUi();
    }

    private void SaveFolderListsToConfig()
    {
        _config.SourceHistory = _sourceFolders.History.ToList();
        _config.SourceFavorites = _sourceFolders.Favorites.ToList();
        _config.DestHistory = _destFolders.History.ToList();
        _config.DestFavorites = _destFolders.Favorites.ToList();
    }

    private void RefreshFolderListsUi()
    {
        _suppressFolderListSelection = true;
        try
        {
            SourceFavoritesList.ItemsSource = null;
            SourceHistoryList.ItemsSource = null;
            DestFavoritesList.ItemsSource = null;
            DestHistoryList.ItemsSource = null;

            SourceFavoritesList.ItemsSource = _sourceFolders.Favorites.ToList();
            SourceHistoryList.ItemsSource = _sourceFolders.History.ToList();
            DestFavoritesList.ItemsSource = _destFolders.Favorites.ToList();
            DestHistoryList.ItemsSource = _destFolders.History.ToList();
        }
        finally
        {
            _suppressFolderListSelection = false;
        }
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
        _config.FontSize = _fontSize;
        SaveFolderListsToConfig();
        _config.Save();
    }

    private void SetActivePane(FilePaneControl pane)
    {
        _activePane = pane;
        LeftPane.SetActive(pane == LeftPane);
        RightPane.SetActive(pane == RightPane);
        UpdateStatusBar();
    }

    public void NotifyActivePaneBranchChanged() => UpdateStatusBar();

    private void SwitchActivePane()
    {
        SetActivePane(_activePane == LeftPane ? RightPane : LeftPane);
        _activePane.FocusList();
    }

    private void UpdateStatusBar()
    {
        var paneLabel = _activePane == LeftPane ? "[左]" : "[右]";
        var branch = _activePane.CurrentBranch;
        StatusPathText.Text = branch != null
            ? $"{paneLabel} {_activePane.CurrentPath}   ⎇ {branch}"
            : $"{paneLabel} {_activePane.CurrentPath}";
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

    // ---- フォントサイズ ----

    private void ApplyFontSize(double size)
    {
        _fontSize = Math.Clamp(size, MinFontSize, MaxFontSize);
        LeftPane.ApplyFontSize(_fontSize);
        RightPane.ApplyFontSize(_fontSize);
        LeftTree.ApplyFontSize(_fontSize);
        RightTree.ApplyFontSize(_fontSize);
        LeftTreeLabel.FontSize = _fontSize;
        RightTreeLabel.FontSize = _fontSize;

        var listHeight = Math.Max(48, _fontSize * 5.5);
        var smallFont = Math.Max(9, _fontSize - 2);

        SourceGroupBox.FontSize = _fontSize;
        DestGroupBox.FontSize = _fontSize;
        SourceFolderBox.FontSize = _fontSize;
        DestFolderBox.FontSize = _fontSize;
        BrowseSourceButton.FontSize = _fontSize;
        BrowseDestButton.FontSize = _fontSize;
        SourceFavLabel.FontSize = smallFont;
        SourceHistLabel.FontSize = smallFont;
        DestFavLabel.FontSize = smallFont;
        DestHistLabel.FontSize = smallFont;
        MirrorShortcutHint.FontSize = smallFont;
        SourceFavoritesList.FontSize = _fontSize;
        SourceHistoryList.FontSize = _fontSize;
        DestFavoritesList.FontSize = _fontSize;
        DestHistoryList.FontSize = _fontSize;
        SourceFavoritesList.Height = listHeight;
        SourceHistoryList.Height = listHeight;
        DestFavoritesList.Height = listHeight;
        DestHistoryList.Height = listHeight;

        StatusPathText.FontSize = Math.Max(10, _fontSize - 1);
        StatusHintText.FontSize = smallFont;
        CommandBox.FontSize = _fontSize;
    }

    private void IncreaseFontSize() => ApplyFontSize(_fontSize + FontStep);
    private void DecreaseFontSize() => ApplyFontSize(_fontSize - FontStep);
    private void ResetFontSize() => ApplyFontSize(AppConfig.DefaultFontSize);

    private bool TryHandleFontShortcut(KeyEventArgs e)
    {
        if (!Keyboard.Modifiers.HasFlag(ModifierKeys.Control))
            return false;

        switch (e.Key)
        {
            case Key.OemPlus:
            case Key.Add:
                IncreaseFontSize();
                return true;
            case Key.OemMinus:
            case Key.Subtract:
                DecreaseFontSize();
                return true;
            case Key.D0:
            case Key.NumPad0:
                ResetFontSize();
                return true;
            default:
                return false;
        }
    }

    // ---- キーボード操作 ----

    private static bool IsTextInputFocused() =>
        Keyboard.FocusedElement is TextBoxBase { IsReadOnly: false } or PasswordBox;

    private void Window_PreviewKeyDown(object sender, KeyEventArgs e)
    {
        if (TryHandleFontShortcut(e))
        {
            e.Handled = true;
            return;
        }

        if (IsTextInputFocused())
            return;

        var pane = _activePane;
        var ctrl = Keyboard.Modifiers.HasFlag(ModifierKeys.Control);
        var key = e.Key;

        switch (key)
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
            case Key.H:
                pane.GoParent();
                break;
            case Key.J:
                pane.MoveCursor(1);
                break;
            case Key.K:
                pane.MoveCursor(-1);
                break;
            case Key.L:
                pane.OpenSelected();
                break;
            case Key.G when Keyboard.Modifiers.HasFlag(ModifierKeys.Shift):
                pane.CursorToEnd();
                break;
            case Key.G:
                pane.CursorToStart();
                break;
            case Key.D1:
            case Key.NumPad1:
                pane.CopyFileNameToClipboard();
                break;
            case Key.D2:
            case Key.NumPad2:
                pane.CopyFullPathToClipboard();
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
            case "~":
                pane.GoHome();
                break;
            case "1":
            case "１":
            case "ぬ":
                pane.CopyFileNameToClipboard();
                break;
            case "2":
            case "２":
            case "ふ":
                pane.CopyFullPathToClipboard();
                break;
            case " ":
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
        InputMethod.SetIsInputMethodEnabled(CommandBox, true);
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

    private void SetSourceFolder(string path, bool addToHistory = true)
    {
        SourceFolderBox.Text = path;
        _config.SourceFolder = path;
        if (addToHistory)
        {
            _sourceFolders.AddToHistory(path);
            RefreshFolderListsUi();
            SaveFolderListsToConfig();
        }
        _config.Save();
    }

    private void SetDestFolder(string path, bool addToHistory = true)
    {
        DestFolderBox.Text = path;
        _config.DestFolder = path;
        if (addToHistory)
        {
            _destFolders.AddToHistory(path);
            RefreshFolderListsUi();
            SaveFolderListsToConfig();
        }
        _config.Save();
    }

    private void BrowseSource_Click(object sender, RoutedEventArgs e)
    {
        if (BrowseFolder("コピー元のフォルダを選択", SourceFolderBox.Text) is { } path)
            SetSourceFolder(path);
    }

    private void BrowseDest_Click(object sender, RoutedEventArgs e)
    {
        if (BrowseFolder("コピー先のフォルダを選択", DestFolderBox.Text) is { } path)
            SetDestFolder(path);
    }

    private void SourceFavoritesList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_suppressFolderListSelection || SourceFavoritesList.SelectedItem is not string path)
            return;
        SetSourceFolder(path, addToHistory: false);
        SourceFavoritesList.SelectedItem = null;
    }

    private void SourceHistoryList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_suppressFolderListSelection || SourceHistoryList.SelectedItem is not string path)
            return;
        SetSourceFolder(path, addToHistory: false);
        SourceHistoryList.SelectedItem = null;
    }

    private void DestFavoritesList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_suppressFolderListSelection || DestFavoritesList.SelectedItem is not string path)
            return;
        SetDestFolder(path, addToHistory: false);
        DestFavoritesList.SelectedItem = null;
    }

    private void DestHistoryList_SelectionChanged(object sender, SelectionChangedEventArgs e)
    {
        if (_suppressFolderListSelection || DestHistoryList.SelectedItem is not string path)
            return;
        SetDestFolder(path, addToHistory: false);
        DestHistoryList.SelectedItem = null;
    }

    private void FolderList_PreviewMouseRightButtonDown(object sender, MouseButtonEventArgs e)
    {
        if (sender is not ListBox listBox)
            return;

        var item = ItemsControl.ContainerFromElement(listBox, e.OriginalSource as DependencyObject) as ListBoxItem;
        if (item == null)
            return;

        item.IsSelected = true;
        var path = item.Content as string;
        if (path == null)
            return;

        var tag = listBox.Tag as string ?? "";
        var isSource = tag.StartsWith("source", StringComparison.Ordinal);
        var isFavorite = tag.EndsWith("fav", StringComparison.Ordinal);
        var manager = isSource ? _sourceFolders : _destFolders;

        var menu = new ContextMenu();
        var selectItem = new MenuItem { Header = isSource ? "コピー元に設定" : "コピー先に設定" };
        selectItem.Click += (_, _) =>
        {
            if (isSource)
                SetSourceFolder(path);
            else
                SetDestFolder(path);
        };
        menu.Items.Add(selectItem);

        if (isFavorite)
        {
            var removeFav = new MenuItem { Header = "お気に入りから削除" };
            removeFav.Click += (_, _) =>
            {
                manager.RemoveFromFavorites(path);
                RefreshFolderListsUi();
                SaveFolderListsToConfig();
            };
            menu.Items.Add(removeFav);
        }
        else
        {
            var addFav = new MenuItem { Header = "お気に入りに追加" };
            addFav.Click += (_, _) =>
            {
                manager.AddToFavorite(path);
                RefreshFolderListsUi();
                SaveFolderListsToConfig();
                ToastWindow.Show("お気に入り", $"登録しました:\n{path}", seconds: 2);
            };
            menu.Items.Add(addFav);
        }

        menu.IsOpen = true;
        e.Handled = true;
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

    // ---- メニューハンドラ ----

    private void MenuQuit_Click(object sender, RoutedEventArgs e) => Close();
    private void MenuRefresh_Click(object sender, RoutedEventArgs e) { _activePane.Refresh(); RefreshTrees(); }
    private void MenuCopy_Click(object sender, RoutedEventArgs e) => _activePane.CopySelectionToClipboard(cut: false);
    private void MenuCut_Click(object sender, RoutedEventArgs e) => _activePane.CopySelectionToClipboard(cut: true);
    private void MenuPaste_Click(object sender, RoutedEventArgs e) => _activePane.PasteFromClipboard();
    private void MenuCopyFileName_Click(object sender, RoutedEventArgs e) => _activePane.CopyFileNameToClipboard();
    private void MenuCopyFullPath_Click(object sender, RoutedEventArgs e) => _activePane.CopyFullPathToClipboard();
    private void MenuGoParent_Click(object sender, RoutedEventArgs e) => _activePane.GoParent();
    private void MenuGoHome_Click(object sender, RoutedEventArgs e) => _activePane.GoHome();
    private void MenuActivateLeft_Click(object sender, RoutedEventArgs e) { SetActivePane(LeftPane); LeftPane.FocusList(); }
    private void MenuActivateRight_Click(object sender, RoutedEventArgs e) { SetActivePane(RightPane); RightPane.FocusList(); }
    private void MenuCopyToOther_Click(object sender, RoutedEventArgs e) => _activePane.CopySelectedToOtherPane();
    private void MenuMoveToOther_Click(object sender, RoutedEventArgs e) => _activePane.MoveSelectedToOtherPane();
    private void MenuDelete_Click(object sender, RoutedEventArgs e) => _activePane.DeleteSelected();
    private void MenuRename_Click(object sender, RoutedEventArgs e) => _activePane.RenameSelected();
    private void MenuNewFolder_Click(object sender, RoutedEventArgs e) => _activePane.CreateNewFolder();
    private void MenuNewFile_Click(object sender, RoutedEventArgs e) => _activePane.CreateNewFile();
    private void MenuToggleHidden_Click(object sender, RoutedEventArgs e) => _activePane.ToggleHidden();
    private void MenuFileInfo_Click(object sender, RoutedEventArgs e) => _activePane.ShowFileInfo();
    private void MenuGitPull_Click(object sender, RoutedEventArgs e) => _activePane.GitPull();
    private void MenuFontIncrease_Click(object sender, RoutedEventArgs e) => IncreaseFontSize();
    private void MenuFontDecrease_Click(object sender, RoutedEventArgs e) => DecreaseFontSize();
    private void MenuFontReset_Click(object sender, RoutedEventArgs e) => ResetFontSize();
    private void MenuMirrorForward_Click(object sender, RoutedEventArgs e) => RunMirror(forward: true);
    private void MenuMirrorBackward_Click(object sender, RoutedEventArgs e) => RunMirror(forward: false);
    private void MenuHelp_Click(object sender, RoutedEventArgs e) =>
        OutputWindow.ShowText(this, "ヘルプ — Mdir for Windows", HelpText);

    private void MenuSort_Click(object sender, RoutedEventArgs e)
    {
        if (sender is MenuItem { Tag: string tag })
        {
            _activePane.SetSort(tag switch
            {
                "size" => SortField.Size,
                "date" => SortField.Date,
                "ext" => SortField.Ext,
                _ => SortField.Name,
            });
        }
    }
}

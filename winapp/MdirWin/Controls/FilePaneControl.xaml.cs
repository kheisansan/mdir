using System.Collections.ObjectModel;
using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using MdirWin.Core;
using MdirWin.Dialogs;

namespace MdirWin.Controls;

public enum SortField { Name, Size, Date, Ext }

public partial class FilePaneControl : UserControl
{
    private readonly ObservableCollection<FileEntry> _entries = new();
    private SortField _sortField = SortField.Name;
    private bool _showHidden;
    private string _findQuery = "";

    /// <summary>ユーザー操作でこのペインがアクティブになったときに発火。</summary>
    public event EventHandler? ActivatedByUser;

    /// <summary>表示中フォルダが変わったとき（更新含む）に発火。</summary>
    public event EventHandler<string>? PathChanged;

    /// <summary>反対側ペインを返すデリゲート(MainWindow が設定)。</summary>
    public Func<FilePaneControl?>? GetOtherPane { get; set; }

    public string CurrentPath { get; private set; } = "";

    /// <summary>現在のフォルダが属する Git ブランチ名。リポジトリ外なら null。</summary>
    public string? CurrentBranch { get; private set; }

    public FilePaneControl()
    {
        InitializeComponent();
        FileList.ItemsSource = _entries;
        // ナビゲーション中は IME を無効化し、h/j/k/l 等が日本語入力モードでも効くようにする
        InputMethod.SetIsInputMethodEnabled(FileList, false);
    }

    public void ApplyFontSize(double size)
    {
        FileList.FontSize = size;
        PathBox.FontSize = size;
        StatusText.FontSize = Math.Max(9, size - 2);
        BranchText.FontSize = Math.Max(9, size - 2);
    }

    public void SetActive(bool active)
    {
        PaneBorder.BorderBrush = active
            ? (Brush)FindResource("AccentBrush")
            : (Brush)FindResource("PaneInactiveBorder");
    }

    public void FocusList() => FileList.Focus();

    // ---- 表示・ナビゲーション ----

    public void NavigateTo(string path)
    {
        try
        {
            var full = Path.GetFullPath(path);
            if (!Directory.Exists(full))
            {
                ToastWindow.Show("エラー", $"フォルダが見つかりません: {full}", isError: true);
                return;
            }

            // 同じフォルダの再読み込みならカーソル位置を維持する
            var keepName = string.Equals(full, CurrentPath, StringComparison.OrdinalIgnoreCase)
                ? (FileList.SelectedItem as FileEntry)?.Name
                : null;

            var dir = new DirectoryInfo(full);
            var dirs = dir.EnumerateDirectories().Select(FileEntry.FromFileSystemInfo);
            var files = dir.EnumerateFiles().Select(FileEntry.FromFileSystemInfo);
            if (!_showHidden)
            {
                dirs = dirs.Where(e => !e.IsHidden);
                files = files.Where(e => !e.IsHidden);
            }

            _entries.Clear();
            if (dir.Parent != null)
            {
                _entries.Add(new FileEntry
                {
                    FullPath = dir.Parent.FullName,
                    Name = "..",
                    IsDirectory = true,
                    IsParentLink = true,
                });
            }
            foreach (var e in SortEntries(dirs))
                _entries.Add(e);
            foreach (var e in SortEntries(files))
                _entries.Add(e);

            CurrentPath = full;
            PathBox.Text = full;
            CurrentBranch = GitHelper.GetBranchName(full);
            UpdateStatus();
            (Window.GetWindow(this) as MainWindow)?.NotifyActivePaneBranchChanged();

            var restored = keepName != null
                ? _entries.FirstOrDefault(e => e.Name == keepName)
                : null;
            FileList.SelectedIndex = restored != null ? _entries.IndexOf(restored) : 0;
            if (FileList.SelectedItem != null)
                FileList.ScrollIntoView(FileList.SelectedItem);

            PathChanged?.Invoke(this, full);
        }
        catch (Exception ex)
        {
            ToastWindow.Show("エラー", ex.Message, isError: true);
        }
    }

    private IEnumerable<FileEntry> SortEntries(IEnumerable<FileEntry> source) => _sortField switch
    {
        SortField.Size => source.OrderBy(e => e.Size).ThenBy(e => e.Name, StringComparer.OrdinalIgnoreCase),
        SortField.Date => source.OrderBy(e => e.Modified).ThenBy(e => e.Name, StringComparer.OrdinalIgnoreCase),
        SortField.Ext => source.OrderBy(e => e.Extension, StringComparer.OrdinalIgnoreCase)
                               .ThenBy(e => e.Name, StringComparer.OrdinalIgnoreCase),
        _ => source.OrderBy(e => e.Name, StringComparer.OrdinalIgnoreCase),
    };

    public void Refresh()
    {
        if (!string.IsNullOrEmpty(CurrentPath))
            NavigateTo(CurrentPath);
    }

    private void UpdateStatus()
    {
        var dirCount = _entries.Count(e => e.IsDirectory && !e.IsParentLink);
        var fileCount = _entries.Count(e => !e.IsDirectory);
        var marked = _entries.Count(e => e.IsMarked);
        var hidden = _showHidden ? " | 隠しファイル表示中" : "";
        var markText = marked > 0 ? $" | マーク {marked}" : "";
        StatusText.Text = $"{dirCount} フォルダ / {fileCount} ファイル{markText}{hidden}";
        BranchText.Text = CurrentBranch != null ? $"⎇ {CurrentBranch}" : "";
    }

    public void GoParent()
    {
        var parent = Directory.GetParent(CurrentPath);
        if (parent != null)
            NavigateTo(parent.FullName);
    }

    public void GoHome() =>
        NavigateTo(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile));

    public void OpenSelected()
    {
        if (FileList.SelectedItem is FileEntry entry)
            OpenEntry(entry);
    }

    private void OpenEntry(FileEntry entry)
    {
        if (entry.IsDirectory)
        {
            NavigateTo(entry.FullPath);
        }
        else
        {
            try
            {
                System.Diagnostics.Process.Start(
                    new System.Diagnostics.ProcessStartInfo(entry.FullPath) { UseShellExecute = true });
            }
            catch (Exception ex)
            {
                ToastWindow.Show("エラー", ex.Message, isError: true);
            }
        }
    }

    // ---- カーソル移動 ----

    public void MoveCursor(int delta)
    {
        if (_entries.Count == 0)
            return;
        var index = FileList.SelectedIndex < 0 ? 0 : FileList.SelectedIndex + delta;
        FileList.SelectedIndex = Math.Clamp(index, 0, _entries.Count - 1);
        FileList.ScrollIntoView(FileList.SelectedItem);
    }

    public void PageMove(int direction) => MoveCursor(direction * PageSize());

    private int PageSize()
    {
        var rows = (int)(FileList.ActualHeight / 22) - 1;
        return Math.Max(rows, 5);
    }

    public void CursorToStart()
    {
        if (_entries.Count == 0)
            return;
        FileList.SelectedIndex = 0;
        FileList.ScrollIntoView(FileList.SelectedItem);
    }

    public void CursorToEnd()
    {
        if (_entries.Count == 0)
            return;
        FileList.SelectedIndex = _entries.Count - 1;
        FileList.ScrollIntoView(FileList.SelectedItem);
    }

    // ---- マーク ----

    public void ToggleMarkAtCursor()
    {
        if (FileList.SelectedItem is FileEntry { IsParentLink: false } entry)
            entry.IsMarked = !entry.IsMarked;
        MoveCursor(1);
        UpdateStatus();
    }

    public void ToggleMarkAll()
    {
        var targets = _entries.Where(e => !e.IsParentLink).ToList();
        var anyMarked = targets.Any(e => e.IsMarked);
        foreach (var e in targets)
            e.IsMarked = !anyMarked;
        UpdateStatus();
    }

    /// <summary>操作対象：マークがあればマーク済み、なければ選択中の項目。</summary>
    private List<FileEntry> TargetEntries()
    {
        var marked = _entries.Where(e => e.IsMarked).ToList();
        if (marked.Count > 0)
            return marked;
        return FileList.SelectedItems.Cast<FileEntry>().Where(e => !e.IsParentLink).ToList();
    }

    private Window OwnerWindow => Window.GetWindow(this)!;

    public void CopySelectionToClipboard(bool cut)
    {
        var selected = TargetEntries();
        if (selected.Count == 0)
            return;
        ClipboardHelper.SetFileDropList(selected.Select(s => s.FullPath), cut);
        ToastWindow.Show(cut ? "切り取り" : "コピー", $"{selected.Count} 件をクリップボードに入れました");
    }

    /// <summary>現在表示中のフォルダ名をクリップボードへコピー（1 キー）。</summary>
    public void CopyFileNameToClipboard()
    {
        if (string.IsNullOrEmpty(CurrentPath))
            return;

        var name = Path.GetFileName(Path.TrimEndingDirectorySeparator(CurrentPath));
        if (string.IsNullOrEmpty(name))
            name = CurrentPath;

        Clipboard.SetText(name);
        ToastWindow.Show("コピー", $"フォルダ名をコピーしました: {name}", seconds: 2);
    }

    /// <summary>現在表示中のフォルダのフルパスをクリップボードへコピー（2 キー）。</summary>
    public void CopyFullPathToClipboard()
    {
        if (string.IsNullOrEmpty(CurrentPath))
            return;

        Clipboard.SetText(CurrentPath);
        ToastWindow.Show("コピー", $"フォルダのフルパスをコピーしました:\n{CurrentPath}", seconds: 2);
    }

    public void PasteFromClipboard()
    {
        var (paths, isCut) = ClipboardHelper.GetFileDropList();
        if (paths.Count == 0)
            return;

        var errors = new List<string>();
        foreach (var p in paths)
        {
            try
            {
                if (isCut)
                    FileOperations.MoveIntoDirectory(p, CurrentPath);
                else
                    FileOperations.CopyIntoDirectory(p, CurrentPath);
            }
            catch (Exception ex)
            {
                errors.Add($"{Path.GetFileName(p)}: {ex.Message}");
            }
        }

        if (isCut)
            Clipboard.Clear();

        RefreshBothPanes();
        if (errors.Count > 0)
            ToastWindow.Show("貼り付けエラー", string.Join("\n", errors.Take(5)), isError: true);
        else
            ToastWindow.Show("貼り付け", $"{paths.Count} 件を貼り付けました");
    }

    // ---- ファイル操作 ----

    public void CopySelectedToOtherPane() => TransferToOtherPane(move: false);

    public void MoveSelectedToOtherPane() => TransferToOtherPane(move: true);

    private void TransferToOtherPane(bool move)
    {
        var other = GetOtherPane?.Invoke();
        if (other == null || string.IsNullOrEmpty(other.CurrentPath))
            return;

        var selected = TargetEntries();
        if (selected.Count == 0)
            return;

        var errors = new List<string>();
        foreach (var entry in selected)
        {
            try
            {
                if (move)
                    FileOperations.MoveIntoDirectory(entry.FullPath, other.CurrentPath);
                else
                    FileOperations.CopyIntoDirectory(entry.FullPath, other.CurrentPath);
            }
            catch (Exception ex)
            {
                errors.Add($"{entry.Name}: {ex.Message}");
            }
        }

        RefreshBothPanes();
        if (errors.Count > 0)
            ToastWindow.Show(move ? "移動エラー" : "コピーエラー", string.Join("\n", errors.Take(5)), isError: true);
        else
            ToastWindow.Show(move ? "移動" : "コピー", $"{selected.Count} 件を {other.CurrentPath} へ{(move ? "移動" : "コピー")}しました");
    }

    public void DeleteSelected()
    {
        var selected = TargetEntries();
        if (selected.Count == 0)
            return;

        var names = string.Join("\n", selected.Take(8).Select(s => s.Name));
        var result = MessageBox.Show(OwnerWindow,
            $"以下の {selected.Count} 件をゴミ箱に移動しますか？\n\n{names}",
            "削除の確認", MessageBoxButton.YesNo, MessageBoxImage.Question);
        if (result != MessageBoxResult.Yes)
            return;

        var errors = new List<string>();
        foreach (var entry in selected)
        {
            try
            {
                if (entry.IsDirectory)
                    Microsoft.VisualBasic.FileIO.FileSystem.DeleteDirectory(
                        entry.FullPath,
                        Microsoft.VisualBasic.FileIO.UIOption.OnlyErrorDialogs,
                        Microsoft.VisualBasic.FileIO.RecycleOption.SendToRecycleBin);
                else
                    Microsoft.VisualBasic.FileIO.FileSystem.DeleteFile(
                        entry.FullPath,
                        Microsoft.VisualBasic.FileIO.UIOption.OnlyErrorDialogs,
                        Microsoft.VisualBasic.FileIO.RecycleOption.SendToRecycleBin);
            }
            catch (Exception ex)
            {
                errors.Add($"{entry.Name}: {ex.Message}");
            }
        }

        RefreshBothPanes();
        if (errors.Count > 0)
            ToastWindow.Show("削除エラー", string.Join("\n", errors.Take(5)), isError: true);
    }

    public void RenameSelected()
    {
        if (FileList.SelectedItem is not FileEntry { IsParentLink: false } entry)
            return;

        var newName = InputDialog.Show(OwnerWindow, "名前の変更", "新しい名前:", entry.Name);
        if (string.IsNullOrWhiteSpace(newName) || newName == entry.Name)
            return;

        try
        {
            var target = Path.Combine(Path.GetDirectoryName(entry.FullPath)!, newName);
            if (entry.IsDirectory)
                Directory.Move(entry.FullPath, target);
            else
                File.Move(entry.FullPath, target);
            RefreshBothPanes();
        }
        catch (Exception ex)
        {
            ToastWindow.Show("名前の変更エラー", ex.Message, isError: true);
        }
    }

    public void CreateNewFolder(string? name = null)
    {
        name ??= InputDialog.Show(OwnerWindow, "新規フォルダ", "フォルダ名:", "新しいフォルダ");
        if (string.IsNullOrWhiteSpace(name))
            return;

        try
        {
            var target = FileOperations.GetUniquePath(Path.Combine(CurrentPath, name), isDirectory: true);
            Directory.CreateDirectory(target);
            RefreshBothPanes();
        }
        catch (Exception ex)
        {
            ToastWindow.Show("フォルダ作成エラー", ex.Message, isError: true);
        }
    }

    public void CreateNewFile(string? name = null)
    {
        name ??= InputDialog.Show(OwnerWindow, "新規ファイル", "ファイル名:", "新しいファイル.txt");
        if (string.IsNullOrWhiteSpace(name))
            return;

        try
        {
            var target = FileOperations.GetUniquePath(Path.Combine(CurrentPath, name), isDirectory: false);
            File.Create(target).Dispose();
            RefreshBothPanes();
        }
        catch (Exception ex)
        {
            ToastWindow.Show("ファイル作成エラー", ex.Message, isError: true);
        }
    }

    // ---- 情報・ソート・表示 ----

    public void ShowFileInfo()
    {
        if (FileList.SelectedItem is not FileEntry { IsParentLink: false } entry)
            return;

        try
        {
            FileSystemInfo info = entry.IsDirectory
                ? new DirectoryInfo(entry.FullPath)
                : new FileInfo(entry.FullPath);
            var size = entry.IsDirectory ? "<DIR>" : $"{entry.Size:N0} bytes ({FileEntry.FormatSize(entry.Size)})";
            MessageBox.Show(OwnerWindow,
                $"名前: {entry.Name}\n" +
                $"パス: {entry.FullPath}\n" +
                $"サイズ: {size}\n" +
                $"作成日時: {info.CreationTime:yyyy-MM-dd HH:mm:ss}\n" +
                $"更新日時: {info.LastWriteTime:yyyy-MM-dd HH:mm:ss}\n" +
                $"属性: {info.Attributes}",
                "ファイル情報", MessageBoxButton.OK, MessageBoxImage.Information);
        }
        catch (Exception ex)
        {
            ToastWindow.Show("エラー", ex.Message, isError: true);
        }
    }

    public void SetSort(SortField field)
    {
        _sortField = field;
        Refresh();
        var label = field switch
        {
            SortField.Size => "サイズ",
            SortField.Date => "更新日時",
            SortField.Ext => "拡張子",
            _ => "名前",
        };
        ToastWindow.Show("ソート", $"{label}順に並べ替えました", seconds: 1.5);
    }

    public void ShowSortMenu()
    {
        var menu = new ContextMenu { PlacementTarget = FileList };
        foreach (var (label, field) in new[]
                 {
                     ("名前 (name)", SortField.Name),
                     ("サイズ (size)", SortField.Size),
                     ("更新日時 (date)", SortField.Date),
                     ("拡張子 (ext)", SortField.Ext),
                 })
        {
            var item = new MenuItem { Header = label };
            var f = field;
            item.Click += (_, _) => SetSort(f);
            menu.Items.Add(item);
        }
        menu.IsOpen = true;
    }

    public void ToggleHidden()
    {
        _showHidden = !_showHidden;
        Refresh();
        ToastWindow.Show("表示切替", _showHidden ? "隠しファイルを表示します" : "隠しファイルを非表示にします", seconds: 1.5);
    }

    // ---- 検索 (:find, o/p) ----

    public void Find(string query)
    {
        _findQuery = query;
        var matches = FindMatches();
        if (matches.Count == 0)
        {
            ToastWindow.Show("検索", $"「{query}」に一致するものはありません", isError: true, seconds: 2);
            return;
        }
        JumpToIndex(matches[0]);
        ToastWindow.Show("検索", $"{matches.Count} 件ヒット (o: 前へ / p: 次へ)", seconds: 2);
    }

    public void FindNext() => FindStep(+1);

    public void FindPrev() => FindStep(-1);

    private void FindStep(int direction)
    {
        var matches = FindMatches();
        if (matches.Count == 0)
            return;

        var cursor = FileList.SelectedIndex;
        var next = direction > 0
            ? matches.FirstOrDefault(i => i > cursor, matches[0])
            : matches.LastOrDefault(i => i < cursor, matches[^1]);
        JumpToIndex(next);
    }

    private List<int> FindMatches()
    {
        if (string.IsNullOrEmpty(_findQuery))
            return new List<int>();
        return _entries
            .Select((e, i) => (e, i))
            .Where(t => !t.e.IsParentLink && t.e.Name.Contains(_findQuery, StringComparison.OrdinalIgnoreCase))
            .Select(t => t.i)
            .ToList();
    }

    private void JumpToIndex(int index)
    {
        FileList.SelectedIndex = index;
        FileList.ScrollIntoView(FileList.SelectedItem);
    }

    // ---- Git ----

    public async void GitPull()
    {
        if (CurrentBranch == null)
        {
            ToastWindow.Show("Git", "Git リポジトリではありません。", isError: true, seconds: 2);
            return;
        }

        ToastWindow.Show("Git", $"git pull 実行中... (⎇ {CurrentBranch})", seconds: 2);
        try
        {
            var (code, output) = await ProcessRunner.RunAsync("git", "pull", CurrentPath);
            var summary = output.Length > 300 ? output[..300] + "..." : output;
            ToastWindow.Show(code == 0 ? "Git Pull 完了" : "Git Pull 失敗", summary, isError: code != 0, seconds: 5);
            Refresh();
        }
        catch (Exception ex)
        {
            ToastWindow.Show("Git Pull エラー", ex.Message, isError: true, seconds: 5);
        }
    }

    private void RefreshBothPanes()
    {
        Refresh();
        GetOtherPane?.Invoke()?.Refresh();
        (Window.GetWindow(this) as MainWindow)?.RefreshTrees();
    }

    // ---- UI イベント ----

    private void UpButton_Click(object sender, RoutedEventArgs e) => GoParent();

    private void PathBox_KeyDown(object sender, KeyEventArgs e)
    {
        if (e.Key == Key.Enter)
        {
            NavigateTo(PathBox.Text);
            FileList.Focus();
        }
    }

    private void FileList_MouseDoubleClick(object sender, MouseButtonEventArgs e) => OpenSelected();

    private void FileList_GotKeyboardFocus(object sender, KeyboardFocusChangedEventArgs e) =>
        ActivatedByUser?.Invoke(this, EventArgs.Empty);

    private void FileList_PreviewMouseDown(object sender, MouseButtonEventArgs e) =>
        ActivatedByUser?.Invoke(this, EventArgs.Empty);

    // ---- コンテキストメニュー ----

    private void ContextMenu_Opened(object sender, RoutedEventArgs e)
    {
        var hasSelection = TargetEntries().Count > 0;
        var singleSelected = FileList.SelectedItem is FileEntry { IsParentLink: false };
        MenuOpen.IsEnabled = singleSelected;
        MenuInfo.IsEnabled = singleSelected;
        MenuCopy.IsEnabled = hasSelection;
        MenuCut.IsEnabled = hasSelection;
        MenuCopyToOther.IsEnabled = hasSelection;
        MenuMoveToOther.IsEnabled = hasSelection;
        MenuRename.IsEnabled = singleSelected;
        MenuDelete.IsEnabled = hasSelection;
        MenuPaste.IsEnabled = ClipboardHelper.HasFiles();
    }

    private void MenuOpen_Click(object sender, RoutedEventArgs e) => OpenSelected();
    private void MenuInfo_Click(object sender, RoutedEventArgs e) => ShowFileInfo();
    private void MenuCopy_Click(object sender, RoutedEventArgs e) => CopySelectionToClipboard(cut: false);
    private void MenuCut_Click(object sender, RoutedEventArgs e) => CopySelectionToClipboard(cut: true);
    private void MenuPaste_Click(object sender, RoutedEventArgs e) => PasteFromClipboard();
    private void MenuCopyToOther_Click(object sender, RoutedEventArgs e) => CopySelectedToOtherPane();
    private void MenuMoveToOther_Click(object sender, RoutedEventArgs e) => MoveSelectedToOtherPane();
    private void MenuRename_Click(object sender, RoutedEventArgs e) => RenameSelected();
    private void MenuDelete_Click(object sender, RoutedEventArgs e) => DeleteSelected();
    private void MenuNewFolder_Click(object sender, RoutedEventArgs e) => CreateNewFolder();
    private void MenuNewFile_Click(object sender, RoutedEventArgs e) => CreateNewFile();
    private void MenuRefresh_Click(object sender, RoutedEventArgs e) => Refresh();

    private void MenuToggleHidden_Click(object sender, RoutedEventArgs e) => ToggleHidden();

    private void MenuSort_Click(object sender, RoutedEventArgs e)
    {
        if (sender is MenuItem { Tag: string tag })
        {
            SetSort(tag switch
            {
                "size" => SortField.Size,
                "date" => SortField.Date,
                "ext" => SortField.Ext,
                _ => SortField.Name,
            });
        }
    }
}

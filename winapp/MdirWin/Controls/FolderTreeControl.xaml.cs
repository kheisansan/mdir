using System.IO;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;
using MdirWin.Core;
using MdirWin.Dialogs;

namespace MdirWin.Controls;

/// <summary>
/// 遅延展開するフォルダツリー。ドライブをルートとして表示する。
/// </summary>
public partial class FolderTreeControl : UserControl
{
    private static readonly object LoadingPlaceholder = new();
    private bool _suppressSelection;

    /// <summary>ユーザーがツリー上のフォルダをクリック（選択）したときに発火。</summary>
    public event EventHandler<string>? FolderSelected;

    /// <summary>ツリー操作でファイルシステムが変化したときに発火。</summary>
    public event EventHandler? TreeChanged;

    public FolderTreeControl()
    {
        InitializeComponent();
        LoadRoots();
    }

    public void LoadRoots()
    {
        Tree.Items.Clear();
        foreach (var drive in DriveInfo.GetDrives().Where(d => d.IsReady))
        {
            var label = string.IsNullOrEmpty(drive.VolumeLabel)
                ? drive.Name
                : $"{drive.Name.TrimEnd('\\')} ({drive.VolumeLabel})";
            var item = CreateItem(label, drive.RootDirectory.FullName, "💾");
            Tree.Items.Add(item);
        }
    }

    /// <summary>ツリー全体を再読み込みし、展開状態をできる限り復元する。</summary>
    public void ReloadPreservingExpansion()
    {
        var expanded = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        CollectExpanded(Tree.Items, expanded);
        var selectedPath = (Tree.SelectedItem as TreeViewItem)?.Tag as string;

        _suppressSelection = true;
        try
        {
            LoadRoots();
            foreach (var path in expanded.OrderBy(p => p.Length))
                ExpandToPath(path, select: false);
            if (selectedPath != null && Directory.Exists(selectedPath))
                ExpandToPath(selectedPath, select: true);
        }
        finally
        {
            _suppressSelection = false;
        }
    }

    private static void CollectExpanded(ItemCollection items, HashSet<string> result)
    {
        foreach (var obj in items)
        {
            if (obj is TreeViewItem { IsExpanded: true, Tag: string path } item)
            {
                result.Add(path);
                CollectExpanded(item.Items, result);
            }
        }
    }

    /// <summary>指定パスまでツリーを展開する（存在すれば選択も行う）。</summary>
    public void ExpandToPath(string path, bool select)
    {
        var full = Path.TrimEndingDirectorySeparator(Path.GetFullPath(path));
        TreeViewItem? current = null;
        var items = Tree.Items;

        while (true)
        {
            TreeViewItem? next = null;
            foreach (var obj in items)
            {
                if (obj is TreeViewItem { Tag: string itemPath } item)
                {
                    var trimmed = Path.TrimEndingDirectorySeparator(itemPath);
                    if (string.Equals(trimmed, full, StringComparison.OrdinalIgnoreCase) ||
                        full.StartsWith(trimmed + Path.DirectorySeparatorChar, StringComparison.OrdinalIgnoreCase))
                    {
                        next = item;
                        break;
                    }
                }
            }

            if (next == null)
                break;

            current = next;
            var currentPath = Path.TrimEndingDirectorySeparator((string)current.Tag);
            if (string.Equals(currentPath, full, StringComparison.OrdinalIgnoreCase))
                break;

            EnsureChildrenLoaded(current);
            current.IsExpanded = true;
            items = current.Items;
        }

        if (select && current != null)
        {
            var prev = _suppressSelection;
            _suppressSelection = true;
            try
            {
                current.IsSelected = true;
                current.BringIntoView();
            }
            finally
            {
                _suppressSelection = prev;
            }
        }
    }

    private TreeViewItem CreateItem(string name, string path, string icon = "📁")
    {
        var header = new StackPanel { Orientation = Orientation.Horizontal };
        header.Children.Add(new TextBlock { Text = icon, Margin = new Thickness(0, 0, 4, 0) });
        header.Children.Add(new TextBlock { Text = name });

        var item = new TreeViewItem { Header = header, Tag = path };
        if (HasSubdirectories(path))
            item.Items.Add(LoadingPlaceholder);
        return item;
    }

    private static bool HasSubdirectories(string path)
    {
        try
        {
            return Directory.EnumerateDirectories(path).Any();
        }
        catch
        {
            return false;
        }
    }

    private void EnsureChildrenLoaded(TreeViewItem item)
    {
        if (item.Items.Count != 1 || item.Items[0] != LoadingPlaceholder)
            return;

        item.Items.Clear();
        var path = (string)item.Tag;
        try
        {
            foreach (var dir in Directory.EnumerateDirectories(path).OrderBy(d => d, StringComparer.OrdinalIgnoreCase))
            {
                var info = new DirectoryInfo(dir);
                if ((info.Attributes & (FileAttributes.Hidden | FileAttributes.System)) != 0)
                    continue;
                item.Items.Add(CreateItem(info.Name, info.FullName));
            }
        }
        catch
        {
            // アクセス拒否などは空のまま
        }
    }

    private void Tree_ItemExpanded(object sender, RoutedEventArgs e)
    {
        if (e.OriginalSource is TreeViewItem item)
            EnsureChildrenLoaded(item);
    }

    private void Tree_SelectedItemChanged(object sender, RoutedPropertyChangedEventArgs<object> e)
    {
        if (_suppressSelection)
            return;
        if (e.NewValue is TreeViewItem { Tag: string path })
            FolderSelected?.Invoke(this, path);
    }

    private void Tree_PreviewMouseRightButtonDown(object sender, MouseButtonEventArgs e)
    {
        // 右クリックした項目を選択してからコンテキストメニューを出す
        var element = e.OriginalSource as DependencyObject;
        while (element != null && element is not TreeViewItem)
            element = VisualTreeHelper.GetParent(element);
        if (element is TreeViewItem item)
        {
            item.IsSelected = true;
            e.Handled = false;
        }
    }

    // ---- コンテキストメニュー ----

    private string? SelectedPath => (Tree.SelectedItem as TreeViewItem)?.Tag as string;

    private Window OwnerWindow => Window.GetWindow(this)!;

    private void ContextMenu_Opened(object sender, RoutedEventArgs e)
    {
        var path = SelectedPath;
        var hasSelection = path != null;
        // ドライブルートは名前変更・削除・コピー不可
        var isDriveRoot = hasSelection && string.Equals(Path.GetPathRoot(path!), path, StringComparison.OrdinalIgnoreCase);

        MenuNewSubfolder.IsEnabled = hasSelection;
        MenuRename.IsEnabled = hasSelection && !isDriveRoot;
        MenuDelete.IsEnabled = hasSelection && !isDriveRoot;
        MenuCopyFolder.IsEnabled = hasSelection && !isDriveRoot;
        MenuCopyName.IsEnabled = hasSelection;
        MenuCopyFullPath.IsEnabled = hasSelection;
        MenuPaste.IsEnabled = hasSelection && ClipboardHelper.HasFiles();
    }

    private void MenuNewSubfolder_Click(object sender, RoutedEventArgs e)
    {
        if (SelectedPath is not { } path)
            return;

        var name = InputDialog.Show(OwnerWindow, "サブフォルダの作成", "フォルダ名:", "新しいフォルダ");
        if (string.IsNullOrWhiteSpace(name))
            return;

        try
        {
            var target = FileOperations.GetUniquePath(Path.Combine(path, name), isDirectory: true);
            Directory.CreateDirectory(target);
            NotifyChanged();
        }
        catch (Exception ex)
        {
            ToastWindow.Show("フォルダ作成エラー", ex.Message, isError: true);
        }
    }

    private void MenuRename_Click(object sender, RoutedEventArgs e)
    {
        if (SelectedPath is not { } path)
            return;

        var oldName = Path.GetFileName(Path.TrimEndingDirectorySeparator(path));
        var newName = InputDialog.Show(OwnerWindow, "フォルダ名の変更", "新しい名前:", oldName);
        if (string.IsNullOrWhiteSpace(newName) || newName == oldName)
            return;

        try
        {
            var parent = Path.GetDirectoryName(Path.TrimEndingDirectorySeparator(path))!;
            Directory.Move(path, Path.Combine(parent, newName));
            NotifyChanged();
        }
        catch (Exception ex)
        {
            ToastWindow.Show("名前の変更エラー", ex.Message, isError: true);
        }
    }

    private void MenuDelete_Click(object sender, RoutedEventArgs e)
    {
        if (SelectedPath is not { } path)
            return;

        var result = MessageBox.Show(OwnerWindow,
            $"フォルダをゴミ箱に移動しますか？\n\n{path}",
            "削除の確認", MessageBoxButton.YesNo, MessageBoxImage.Question);
        if (result != MessageBoxResult.Yes)
            return;

        try
        {
            Microsoft.VisualBasic.FileIO.FileSystem.DeleteDirectory(
                path,
                Microsoft.VisualBasic.FileIO.UIOption.OnlyErrorDialogs,
                Microsoft.VisualBasic.FileIO.RecycleOption.SendToRecycleBin);
            NotifyChanged();
        }
        catch (Exception ex)
        {
            ToastWindow.Show("削除エラー", ex.Message, isError: true);
        }
    }

    private void MenuCopyFolder_Click(object sender, RoutedEventArgs e)
    {
        if (SelectedPath is not { } path)
            return;
        ClipboardHelper.SetFileDropList(new[] { path }, cut: false);
        ToastWindow.Show("コピー", $"フォルダをクリップボードに入れました:\n{path}");
    }

    private void MenuCopyName_Click(object sender, RoutedEventArgs e)
    {
        if (SelectedPath is not { } path)
            return;
        var name = Path.GetFileName(Path.TrimEndingDirectorySeparator(path));
        if (string.IsNullOrEmpty(name))
            name = path;
        Clipboard.SetText(name);
        ToastWindow.Show("コピー", $"フォルダ名をコピーしました: {name}");
    }

    private void MenuCopyFullPath_Click(object sender, RoutedEventArgs e)
    {
        if (SelectedPath is not { } path)
            return;
        Clipboard.SetText(path);
        ToastWindow.Show("コピー", $"フルパスをコピーしました:\n{path}");
    }

    private void MenuPaste_Click(object sender, RoutedEventArgs e)
    {
        if (SelectedPath is not { } destDir)
            return;

        var (paths, isCut) = ClipboardHelper.GetFileDropList();
        if (paths.Count == 0)
            return;

        var errors = new List<string>();
        foreach (var p in paths)
        {
            try
            {
                if (isCut)
                    FileOperations.MoveIntoDirectory(p, destDir);
                else
                    FileOperations.CopyIntoDirectory(p, destDir);
            }
            catch (Exception ex)
            {
                errors.Add($"{Path.GetFileName(p)}: {ex.Message}");
            }
        }

        if (isCut)
            Clipboard.Clear();

        NotifyChanged();
        if (errors.Count > 0)
            ToastWindow.Show("貼り付けエラー", string.Join("\n", errors.Take(5)), isError: true);
        else
            ToastWindow.Show("貼り付け", $"{paths.Count} 件を貼り付けました:\n{destDir}");
    }

    private void NotifyChanged()
    {
        ReloadPreservingExpansion();
        TreeChanged?.Invoke(this, EventArgs.Empty);
    }
}

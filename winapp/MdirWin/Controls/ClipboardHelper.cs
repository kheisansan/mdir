using System.IO;
using System.Windows;

namespace MdirWin.Controls;

/// <summary>
/// エクスプローラー互換のファイルクリップボード操作。
/// "Preferred DropEffect" でコピー/切り取りを区別する。
/// </summary>
public static class ClipboardHelper
{
    private const string DropEffectFormat = "Preferred DropEffect";

    public static void SetFileDropList(IEnumerable<string> paths, bool cut)
    {
        var data = new DataObject();
        var list = new System.Collections.Specialized.StringCollection();
        foreach (var p in paths)
            list.Add(p);
        data.SetFileDropList(list);

        // 2 = Move（切り取り）, 5 = Copy|Link — エクスプローラーと同じ形式
        var effect = new MemoryStream(BitConverter.GetBytes(cut ? 2 : 5));
        data.SetData(DropEffectFormat, effect);

        Clipboard.SetDataObject(data, copy: true);
    }

    public static (IReadOnlyList<string> Paths, bool IsCut) GetFileDropList()
    {
        try
        {
            if (!Clipboard.ContainsFileDropList())
                return (Array.Empty<string>(), false);

            var paths = Clipboard.GetFileDropList().Cast<string>().ToList();

            var isCut = false;
            var data = Clipboard.GetDataObject();
            if (data?.GetData(DropEffectFormat) is MemoryStream ms)
            {
                var bytes = new byte[4];
                if (ms.Read(bytes, 0, 4) == 4)
                {
                    var effect = BitConverter.ToInt32(bytes, 0);
                    isCut = (effect & 2) != 0 && (effect & 1) == 0; // Move ビットのみ立っていれば切り取り
                }
            }

            return (paths, isCut);
        }
        catch
        {
            return (Array.Empty<string>(), false);
        }
    }

    public static bool HasFiles()
    {
        try
        {
            return Clipboard.ContainsFileDropList();
        }
        catch
        {
            return false;
        }
    }
}

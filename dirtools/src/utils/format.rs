// utils/format.rs - 表示用フォーマットユーティリティ
//
// ファイルサイズ、日時、パスなどの人間可読形式への変換を提供する。
// 複数の UI ウィジェットから共通利用される。

use chrono::{DateTime, Datelike, Local};
use std::path::Path;
use std::time::SystemTime;

/// ファイルサイズを人間可読形式にフォーマット
///
/// # Examples
/// - 0 → "0B"
/// - 512 → "512B"
/// - 1024 → "1.0KB"
/// - 1048576 → "1.0MB"
pub fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0B".to_string();
    }

    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{}B", bytes)
    } else {
        format!("{:.1}{}", size, UNITS[unit_index])
    }
}

/// SystemTime を表示用文字列にフォーマット
///
/// - 当日: "HH:MM"
/// - 今年: "MM/DD"
/// - 去年以前: "YYYY"
pub fn format_time(time: SystemTime, date_fmt: &str, time_fmt: &str) -> String {
    let datetime: DateTime<Local> = time.into();
    let now = Local::now();

    if datetime.date_naive() == now.date_naive() {
        datetime.format(time_fmt).to_string()
    } else if datetime.year() == now.year() {
        datetime.format(date_fmt).to_string()
    } else {
        datetime.format("%Y").to_string()
    }
}

/// パスを表示用に短縮（ホームを `~` に、長い場合は `...` で省略）
///
/// 表示幅（カラム数）ベースで計算し、UTF-8 文字境界を保証する。
pub fn format_path(path: &Path, max_width: usize) -> String {
    use unicode_width::UnicodeWidthStr;

    let home = dirs::home_dir();
    let display = if let Some(ref home) = home {
        if let Ok(relative) = path.strip_prefix(home) {
            if relative.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~/{}", relative.display())
            }
        } else {
            path.display().to_string()
        }
    } else {
        path.display().to_string()
    };

    let display_width = display.width();
    if display_width <= max_width {
        display
    } else if max_width > 4 {
        // 末尾から max_width - 4 カラム分を表示（".../" プレフィックス付き）
        let target_width = max_width - 4;
        let mut current_width = 0;
        let mut start_byte = display.len();

        // 末尾から文字を辿り、target_width カラムに収まる開始位置を求める
        for (i, c) in display.char_indices().rev() {
            let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if current_width + char_width > target_width {
                break;
            }
            current_width += char_width;
            start_byte = i;
        }

        let truncated = &display[start_byte..];
        format!(".../{}", truncated.trim_start_matches('/'))
    } else {
        // 非常に狭い場合: 先頭から max_width カラム分だけ取得
        let mut current_width = 0;
        let mut end_byte = 0;
        for (i, c) in display.char_indices() {
            let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
            if current_width + char_width > max_width {
                break;
            }
            current_width += char_width;
            end_byte = i + c.len_utf8();
        }
        display[..end_byte].to_string()
    }
}

/// パーミッション表示文字列を生成
///
/// Unix: "-rwxr-xr-x" 形式（モードビットから生成）
/// Windows: モード値が 0 の場合はファイル種別のみ表示
pub fn format_permissions(mode: u32, is_dir: bool, is_symlink: bool) -> String {
    let file_type = if is_symlink {
        'l'
    } else if is_dir {
        'd'
    } else {
        '-'
    };

    // Windows ではモード値が 0 になるため簡易表示
    if mode == 0 {
        let kind = if is_symlink {
            "リンク"
        } else if is_dir {
            "フォルダ"
        } else {
            "ファイル"
        };
        return format!("{} {}", file_type, kind);
    }

    // Unix パーミッションモードを "-rwxr-xr-x" 形式に変換
    format!(
        "{}{}{}{}{}{}{}{}{}{}",
        file_type,
        if mode & 0o400 != 0 { 'r' } else { '-' },
        if mode & 0o200 != 0 { 'w' } else { '-' },
        if mode & 0o100 != 0 { 'x' } else { '-' },
        if mode & 0o040 != 0 { 'r' } else { '-' },
        if mode & 0o020 != 0 { 'w' } else { '-' },
        if mode & 0o010 != 0 { 'x' } else { '-' },
        if mode & 0o004 != 0 { 'r' } else { '-' },
        if mode & 0o002 != 0 { 'w' } else { '-' },
        if mode & 0o001 != 0 { 'x' } else { '-' },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_zero() {
        assert_eq!(format_size(0), "0B");
    }

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(512), "512B");
        assert_eq!(format_size(1), "1B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0KB");
        assert_eq!(format_size(1536), "1.5KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1_048_576), "1.0MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1_073_741_824), "1.0GB");
    }

    #[test]
    fn test_format_permissions() {
        assert_eq!(format_permissions(0o755, false, false), "-rwxr-xr-x");
        assert_eq!(format_permissions(0o644, false, false), "-rw-r--r--");
        assert_eq!(format_permissions(0o755, true, false), "drwxr-xr-x");
        assert_eq!(format_permissions(0o777, false, true), "lrwxrwxrwx");
    }

    #[test]
    fn test_format_path_short() {
        let path = Path::new("/tmp/test");
        assert_eq!(format_path(path, 50), "/tmp/test");
    }
}

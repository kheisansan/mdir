// ime.rs - 入力メソッド (IME) 制御
//
// アプリ起動時に ASCII（ローマ字）入力モードを強制し、
// コマンド入力やファイル名変更時のみ日本語入力を許可する。
// テキスト入力を完了したら再び ASCII モードに戻す。
//
// macOS: Carbon framework の TIS (Text Input Source) API を使用
// Windows/Linux: 現時点では no-op（将来対応）

/// アプリ起動時に呼ぶ。現在の入力ソースを保存し、ASCII モードに切り替える。
pub fn init() {
    platform::init();
}

/// ASCII（ローマ字）入力モードを強制する
///
/// コマンドモードやダイアログから通常モードに戻った際に呼ばれる。
pub fn force_ascii() {
    platform::force_ascii();
}

/// アプリ終了時に呼ぶ。起動前の入力ソースを復元する。
pub fn cleanup() {
    platform::cleanup();
}

// ---------------------------------------------------------------------------
// macOS 実装: Carbon framework の TIS (Text Input Source) API
// ---------------------------------------------------------------------------
//
// TISCopyInputSourceForLanguage("en") で英語キーボードレイアウトを取得し、
// TISSelectInputSource で切り替える。
// macOS 12 以降 deprecated だが、現在も動作し代替 API がないため使用。

#[cfg(target_os = "macos")]
mod platform {
    use std::ffi::c_void;
    use std::sync::Mutex;

    // --- CoreFoundation / Carbon FFI 型定義 ---
    type CFAllocatorRef = *const c_void;
    type CFStringRef = *const c_void;
    type TISInputSourceRef = *const c_void;
    #[allow(dead_code)]
    type OSStatus = i32;

    const K_CF_STRING_ENCODING_UTF8: u32 = 0x0800_0100;

    #[link(name = "Carbon", kind = "framework")]
    extern "C" {
        fn TISCopyCurrentKeyboardInputSource() -> TISInputSourceRef;
        fn TISCopyInputSourceForLanguage(language: CFStringRef) -> TISInputSourceRef;
        fn TISSelectInputSource(source: TISInputSourceRef) -> OSStatus;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFStringCreateWithCString(
            alloc: CFAllocatorRef,
            c_str: *const u8,
            encoding: u32,
        ) -> CFStringRef;
        fn CFRelease(cf: *const c_void);
    }

    /// 起動前の入力ソースを保持（ポインタを usize で保持して Send 要件を回避）
    static SAVED_SOURCE: Mutex<Option<usize>> = Mutex::new(None);

    /// null 終端バイト列から CFString を作成するヘルパー
    unsafe fn cfstr(s: &[u8]) -> CFStringRef {
        CFStringCreateWithCString(std::ptr::null(), s.as_ptr(), K_CF_STRING_ENCODING_UTF8)
    }

    pub fn init() {
        unsafe {
            // 現在の入力ソースを保存（終了時に復元するため）
            let current = TISCopyCurrentKeyboardInputSource();
            if !current.is_null() {
                // TISCopy* で取得 → 所有権あり → cleanup で CFRelease する
                if let Ok(mut saved) = SAVED_SOURCE.lock() {
                    *saved = Some(current as usize);
                }
            }
        }
        force_ascii();
    }

    pub fn force_ascii() {
        unsafe {
            let lang = cfstr(b"en\0");
            if lang.is_null() {
                return;
            }
            let source = TISCopyInputSourceForLanguage(lang);
            if !source.is_null() {
                let _ = TISSelectInputSource(source);
                CFRelease(source);
            }
            CFRelease(lang);
        }
    }

    pub fn cleanup() {
        let saved = SAVED_SOURCE.lock().ok().and_then(|mut guard| guard.take());
        if let Some(addr) = saved {
            unsafe {
                let source = addr as TISInputSourceRef;
                let _ = TISSelectInputSource(source);
                CFRelease(source);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 非 macOS 実装: no-op
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "macos"))]
mod platform {
    pub fn init() {}
    pub fn force_ascii() {}
    pub fn cleanup() {}
}

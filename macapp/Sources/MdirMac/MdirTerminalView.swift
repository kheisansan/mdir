import AppKit
import Carbon.HIToolbox
import SwiftTerm

/// `LocalProcessTerminalView` を継承し、日本語 IME がオンでも
/// ナビゲーション操作（j/k/h/l 等の vi キー）を半角英数字として
/// 即座に受け付けられるようにしたターミナルビュー。
///
/// 仕組み:
/// - `imeAllowed == false`（ナビゲーション状態）のとき、英字・記号キーは
///   IME を経由させず、ASCII-capable キーボードレイアウトで物理キーを
///   ASCII 文字へ直接変換してプロセスへ送る。
///   これにより「ローマ字入力モードのまま j を押す → 変換待ちで止まる」
///   という問題が起きず、すぐに下へ移動できる。
/// - `imeAllowed == true`（ディレクトリ名・コマンド・検索の入力中）のときは
///   IME 経路をそのまま通し、日本語入力を許可する。
///
/// SwiftTerm の `keyDown(with:)` は `public`（`open` ではない）ためサブクラスで
/// 直接オーバーライドできない。そこでローカルキーイベントモニタを使い、
/// 自分がキーウィンドウのファーストレスポンダである間だけキー入力を横取りする。
///
/// `imeAllowed` の値は、埋め込んだ `mdir` 本体が現在のモードを私的 OSC
/// シーケンスで通知してくるのを `TerminalContainerView` が受けて更新する。
final class MdirTerminalView: LocalProcessTerminalView {
    /// テキスト入力（IME 許可）状態かどうか。false ならナビゲーション（ASCII 強制）。
    var imeAllowed: Bool = false {
        didSet {
            guard oldValue != imeAllowed, !imeAllowed else { return }
            // ナビゲーションへ戻る際、未確定の変換中文字が残っていれば破棄する
            inputContext?.discardMarkedText()
        }
    }

    private var keyMonitor: Any?

    // MARK: - フォントサイズ

    /// OS 標準のフォントサイズ。これを初期値かつ下限とする
    /// （ユーザー要望により、デフォルトより小さくはできない）。
    private let minFontSize: CGFloat = NSFont.systemFontSize
    /// 拡大時の上限
    private let maxFontSize: CGFloat = 48
    /// 1 操作あたりの増減幅
    private let fontStep: CGFloat = 1
    /// 現在のフォントサイズ
    private var currentFontSize: CGFloat = NSFont.systemFontSize

    /// フォントサイズが変化するたびに呼ばれる。サイドバー/ミラーコピーパネルの
    /// SwiftUI 側フォントサイズを追従させるために `MdirController` が購読する。
    var onFontSizeChange: ((CGFloat) -> Void)?

    public override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        if window == nil {
            removeKeyMonitor()
        } else {
            installKeyMonitor()
        }
    }

    deinit {
        removeKeyMonitor()
    }

    private func installKeyMonitor() {
        guard keyMonitor == nil else { return }
        keyMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self else { return event }
            return self.interceptKeyDown(event)
        }
    }

    private func removeKeyMonitor() {
        if let keyMonitor {
            NSEvent.removeMonitor(keyMonitor)
            self.keyMonitor = nil
        }
    }

    // MARK: - フォントサイズ操作

    /// 等幅フォントを生成（SF Mono 優先、無ければシステム等幅にフォールバック）。
    private func monospacedFont(ofSize size: CGFloat) -> NSFont {
        NSFont(name: "SF Mono", size: size)
            ?? NSFont.monospacedSystemFont(ofSize: size, weight: .regular)
    }

    /// OS 標準サイズのフォントを適用する。ビュー生成直後に一度呼ぶ。
    func applyDefaultFont() {
        currentFontSize = minFontSize
        font = monospacedFont(ofSize: currentFontSize)
        onFontSizeChange?(currentFontSize)
    }

    /// フォントサイズを 1 段階拡大する（上限あり）。
    func increaseFontSize() {
        setFontSize(currentFontSize + fontStep)
    }

    /// フォントサイズを 1 段階縮小する（OS 標準サイズが下限）。
    func decreaseFontSize() {
        setFontSize(currentFontSize - fontStep)
    }

    /// フォントサイズを OS 標準サイズにリセットする。
    /// SwiftTerm 側に同名 `resetFontSize()` があり衝突するため別名にしている。
    func resetToDefaultFontSize() {
        setFontSize(minFontSize)
    }

    /// フォントサイズを下限〜上限にクランプして適用する。
    /// `font` の設定で SwiftTerm がセル寸法を再計算し、PTY のサイズ
    /// （rows/cols）も追従するため、mdir 本体も自動で再描画される。
    private func setFontSize(_ size: CGFloat) {
        let clamped = min(maxFontSize, max(minFontSize, size))
        guard clamped != currentFontSize else { return }
        currentFontSize = clamped
        font = monospacedFont(ofSize: clamped)
        onFontSizeChange?(clamped)
    }

    /// キーイベントを横取りし、ナビゲーション中の英字・記号キーを ASCII として
    /// 直接プロセスへ送る。消費した場合は nil を返し（IME へ渡さない）、
    /// それ以外は元のイベントをそのまま通常経路へ流す。
    private func interceptKeyDown(_ event: NSEvent) -> NSEvent? {
        // 自分がキーウィンドウのファーストレスポンダのときだけ介入する
        guard let window, window.isKeyWindow, window.firstResponder === self else {
            return event
        }

        // フォントサイズ変更（Cmd + "+"/"-"）。
        // キーボード配列（JIS/ANSI）で +/-/= の物理キー位置が異なるため、
        // 物理キーコードではなく「実際に生成される文字」で判定する。
        // Cmd 併用時は IME が合成しないため、`characters` は配列・Shift を反映した
        // ASCII 記号になり、ローマ字日本語入力モードでも確実に効く。
        // 例: ANSI Cmd+= / Cmd+Shift+=（＝Cmd++）、JIS Cmd+Shift+;（＝+）、テンキー +/- 等。
        if event.modifierFlags.contains(.command) {
            let produced = (event.characters ?? "") + (event.charactersIgnoringModifiers ?? "")
            if produced.contains("+") || produced.contains("=") {
                increaseFontSize()
                return nil
            }
            if produced.contains("-") || produced.contains("_") {
                decreaseFontSize()
                return nil
            }
        }

        // テキスト入力中は通常の IME 経路に委ねる
        if imeAllowed {
            return event
        }
        let flags = event.modifierFlags
        // Command / Control / Option を含むキーは通常処理に委ねる
        // （メニューショートカット・制御シーケンス・メタキー等）
        if flags.contains(.command) || flags.contains(.control) || flags.contains(.option) {
            return event
        }
        // IME を無視し、物理キーから ASCII 印字文字を算出して直接送出する。
        // 矢印・ファンクション・Enter・Tab・Esc 等の非印字キーは nil となり、
        // SwiftTerm 側の正しいエンコードに委ねられる。
        if let ascii = Self.asciiPrintable(forKeyCode: event.keyCode,
                                           shift: flags.contains(.shift)) {
            send(txt: ascii)
            return nil
        }
        return event
    }

    /// 現在の入力ソースに紐づく ASCII-capable キーボードレイアウトを用いて、
    /// 物理キーコードを印字可能 ASCII 文字（0x20〜0x7e）へ変換する。
    /// 日本語 IME が選択されていても下位の ASCII レイアウトで解決するため、
    /// 例えば `j` キーは常に "j"（Shift 併用で "J"）を返す。
    /// 印字可能 ASCII に解決できない（制御文字・関数キー等）の場合は nil。
    private static func asciiPrintable(forKeyCode keyCode: UInt16, shift: Bool) -> String? {
        guard let source = TISCopyCurrentASCIICapableKeyboardLayoutInputSource()?.takeRetainedValue(),
              let layoutPtr = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData) else {
            return nil
        }
        let layoutData = Unmanaged<CFData>.fromOpaque(layoutPtr).takeUnretainedValue() as Data

        // UCKeyTranslate の modifierKeyState は Carbon の修飾子を 8bit 右シフトした値
        let modifierKeyState: UInt32 = shift ? UInt32((shiftKey >> 8) & 0xff) : 0
        var deadKeyState: UInt32 = 0
        var chars = [UniChar](repeating: 0, count: 4)
        var realLength = 0

        let status = layoutData.withUnsafeBytes { raw -> OSStatus in
            guard let base = raw.bindMemory(to: UCKeyboardLayout.self).baseAddress else {
                return OSStatus(paramErr)
            }
            return UCKeyTranslate(
                base,
                keyCode,
                UInt16(kUCKeyActionDown),
                modifierKeyState,
                UInt32(LMGetKbdType()),
                OptionBits(kUCKeyTranslateNoDeadKeysBit),
                &deadKeyState,
                chars.count,
                &realLength,
                &chars
            )
        }

        guard status == noErr, realLength > 0 else { return nil }
        let result = String(utf16CodeUnits: chars, count: realLength)
        // 印字可能 ASCII のみ通す（制御文字・関数キー・マルチバイトは除外）
        guard result.unicodeScalars.allSatisfy({ $0.value >= 0x20 && $0.value <= 0x7e }) else {
            return nil
        }
        return result
    }
}

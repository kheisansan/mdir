import AppKit
import Carbon.HIToolbox

/// Carbon の RegisterEventHotKey によるグローバルショートカット管理。
/// 他のアプリにフォーカスがあっても発火する
/// （winapp/GlobalHotkeys.cs の Win32 RegisterHotKey 相当）。
///
/// Windows キーが無い macOS では、WPF版の Win+Ctrl+Shift+[ / ] に対応する
/// ショートカットとして Control+Option+Command+[ / ] を割り当てる。
final class GlobalHotkeys {
    private static let signature: OSType = 0x6D64_6972 // 'mdir'
    private static let idMirrorForward: UInt32 = 1
    private static let idMirrorBackward: UInt32 = 2

    private var forwardRef: EventHotKeyRef?
    private var backwardRef: EventHotKeyRef?
    private var eventHandler: EventHandlerRef?

    private let onMirrorForward: () -> Void
    private let onMirrorBackward: () -> Void

    private(set) var forwardRegistered = false
    private(set) var backwardRegistered = false

    init(onMirrorForward: @escaping () -> Void, onMirrorBackward: @escaping () -> Void) {
        self.onMirrorForward = onMirrorForward
        self.onMirrorBackward = onMirrorBackward
        install()
    }

    private func install() {
        var eventType = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        let selfPtr = Unmanaged.passUnretained(self).toOpaque()

        InstallEventHandler(GetApplicationEventTarget(), { _, eventRef, userData in
            guard let userData, let eventRef else { return noErr }
            let hotkeys = Unmanaged<GlobalHotkeys>.fromOpaque(userData).takeUnretainedValue()
            var hotKeyID = EventHotKeyID()
            GetEventParameter(
                eventRef, EventParamName(kEventParamDirectObject), EventParamType(typeEventHotKeyID),
                nil, MemoryLayout<EventHotKeyID>.size, nil, &hotKeyID)
            switch hotKeyID.id {
            case GlobalHotkeys.idMirrorForward:
                hotkeys.onMirrorForward()
            case GlobalHotkeys.idMirrorBackward:
                hotkeys.onMirrorBackward()
            default:
                break
            }
            return noErr
        }, 1, &eventType, selfPtr, &eventHandler)

        let mods: UInt32 = UInt32(controlKey | optionKey | cmdKey)

        let fwdID = EventHotKeyID(signature: Self.signature, id: Self.idMirrorForward)
        let fwdStatus = RegisterEventHotKey(
            UInt32(kVK_ANSI_LeftBracket), mods, fwdID, GetApplicationEventTarget(), 0, &forwardRef)
        forwardRegistered = fwdStatus == noErr

        let bwdID = EventHotKeyID(signature: Self.signature, id: Self.idMirrorBackward)
        let bwdStatus = RegisterEventHotKey(
            UInt32(kVK_ANSI_RightBracket), mods, bwdID, GetApplicationEventTarget(), 0, &backwardRef)
        backwardRegistered = bwdStatus == noErr
    }

    deinit {
        if let forwardRef { UnregisterEventHotKey(forwardRef) }
        if let backwardRef { UnregisterEventHotKey(backwardRef) }
        if let eventHandler { RemoveEventHandler(eventHandler) }
    }
}

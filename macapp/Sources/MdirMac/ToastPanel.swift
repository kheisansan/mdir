import AppKit

/// 画面右下に数秒表示して自動で消える通知パネル。
/// winapp/Dialogs/ToastWindow.xaml(.cs) の AppKit 移植。
enum ToastPanel {
    @MainActor private static var current: NSPanel?
    @MainActor private static var dismissTimer: Timer?

    @MainActor
    static func show(title: String, body: String, isError: Bool = false, seconds: Double = 3.0) {
        current?.close()
        dismissTimer?.invalidate()

        let titleLabel = NSTextField(labelWithString: title)
        titleLabel.font = .boldSystemFont(ofSize: 13)
        titleLabel.textColor = .white

        let bodyLabel = NSTextField(wrappingLabelWithString: body)
        bodyLabel.font = .systemFont(ofSize: 12)
        bodyLabel.textColor = NSColor.white.withAlphaComponent(0.9)
        bodyLabel.preferredMaxLayoutWidth = 320

        let stack = NSStackView(views: [titleLabel, bodyLabel])
        stack.orientation = .vertical
        stack.alignment = .leading
        stack.spacing = 4
        stack.edgeInsets = NSEdgeInsets(top: 12, left: 14, bottom: 12, right: 14)
        stack.translatesAutoresizingMaskIntoConstraints = false

        let background = NSVisualEffectView()
        background.material = .hudWindow
        background.state = .active
        background.wantsLayer = true
        background.layer?.cornerRadius = 10
        background.layer?.backgroundColor = isError
            ? NSColor(calibratedRed: 0.55, green: 0.12, blue: 0.12, alpha: 0.92).cgColor
            : NSColor(calibratedWhite: 0.1, alpha: 0.88).cgColor
        background.translatesAutoresizingMaskIntoConstraints = false

        background.addSubview(stack)
        NSLayoutConstraint.activate([
            stack.leadingAnchor.constraint(equalTo: background.leadingAnchor),
            stack.trailingAnchor.constraint(equalTo: background.trailingAnchor),
            stack.topAnchor.constraint(equalTo: background.topAnchor),
            stack.bottomAnchor.constraint(equalTo: background.bottomAnchor),
        ])

        let width: CGFloat = 340
        let panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: width, height: 10),
            styleMask: [.nonactivatingPanel, .borderless],
            backing: .buffered, defer: false)
        panel.isOpaque = false
        panel.backgroundColor = .clear
        panel.hasShadow = true
        panel.level = .floating
        panel.collectionBehavior = [.canJoinAllSpaces, .stationary]
        panel.contentView = background

        let fitting = background.fittingSize
        let height = max(48, fitting.height)
        if let screen = NSScreen.main {
            let area = screen.visibleFrame
            panel.setFrame(
                NSRect(x: area.maxX - width - 16, y: area.minY + 16, width: width, height: height),
                display: true)
        }

        panel.orderFrontRegardless()
        current = panel

        dismissTimer = Timer.scheduledTimer(withTimeInterval: seconds, repeats: false) { _ in
            Task { @MainActor in
                panel.close()
                if current === panel { current = nil }
            }
        }
    }
}

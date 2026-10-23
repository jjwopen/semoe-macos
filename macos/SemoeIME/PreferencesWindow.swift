import AppKit

/// 환경설정 창: 입력 방식, 약어, 키뗌 사용, 조합 종료 타이머.
final class PreferencesWindow: NSObject {
    static let shared = PreferencesWindow()

    private var window: NSWindow?
    private let modePopup = NSPopUpButton()
    private let abbreviations = NSButton(checkboxWithTitle: "약어 사용", target: nil, action: nil)
    private let keyUp = NSButton(checkboxWithTitle: "키를 모두 떼면 확정 (끄면 타이머로만 확정)", target: nil, action: nil)
    private let timeout = NSSlider()
    private let timeoutLabel = NSTextField(labelWithString: "")
    private let tapStatus = NSTextField(labelWithString: "")
    private let permission = NSButton(title: "입력 모니터링 권한 요청…", target: nil, action: nil)

    func show() {
        if window == nil {
            window = makeWindow()
        }
        refresh()
        NSApp.activate(ignoringOtherApps: true)
        window?.center()
        window?.makeKeyAndOrderFront(nil)
    }

    private func makeWindow() -> NSWindow {
        for mode in Preferences.Mode.allCases {
            modePopup.addItem(withTitle: mode.title)
            modePopup.lastItem?.representedObject = mode.rawValue
        }
        modePopup.target = self
        modePopup.action = #selector(modeChanged)
        abbreviations.target = self
        abbreviations.action = #selector(abbreviationsChanged)
        keyUp.target = self
        keyUp.action = #selector(keyUpChanged)
        permission.target = self
        permission.action = #selector(requestPermission)
        permission.bezelStyle = .rounded

        let range = Preferences.timeoutRange
        timeout.minValue = Double(range.lowerBound)
        timeout.maxValue = Double(range.upperBound)
        timeout.numberOfTickMarks = (range.upperBound - range.lowerBound) / Preferences.timeoutStep + 1
        timeout.allowsTickMarkValuesOnly = true
        timeout.target = self
        timeout.action = #selector(timeoutChanged)
        timeoutLabel.font = .monospacedDigitSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)
        tapStatus.textColor = .secondaryLabelColor

        let timeoutRow = NSStackView(views: [timeout, timeoutLabel])
        timeout.widthAnchor.constraint(equalToConstant: 220).isActive = true
        let timeoutHelp = NSTextField(wrappingLabelWithString: "한 번에 모아치기에서 마지막 키를 누른 뒤 이 시간이 지나면 키를 누르고 있어도 확정합니다.")
        timeoutHelp.textColor = .secondaryLabelColor
        timeoutHelp.font = .systemFont(ofSize: NSFont.smallSystemFontSize)

        let grid = NSGridView(views: [
            [NSTextField(labelWithString: "입력 방식"), modePopup],
            [NSGridCell.emptyContentView, abbreviations],
            [NSGridCell.emptyContentView, keyUp],
            [NSGridCell.emptyContentView, tapStatus],
            [NSGridCell.emptyContentView, permission],
            [NSTextField(labelWithString: "조합 종료 타이머"), timeoutRow],
            [NSGridCell.emptyContentView, timeoutHelp],
        ])
        grid.rowSpacing = 10
        grid.columnSpacing = 12
        grid.column(at: 0).xPlacement = .trailing
        grid.translatesAutoresizingMaskIntoConstraints = false
        timeoutHelp.preferredMaxLayoutWidth = 320

        let w = NSWindow(contentRect: NSRect(x: 0, y: 0, width: 480, height: 280),
                         styleMask: [.titled, .closable], backing: .buffered, defer: false)
        w.title = "세모이 환경설정"
        w.isReleasedWhenClosed = false
        let content = NSView()
        content.addSubview(grid)
        NSLayoutConstraint.activate([
            grid.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 20),
            grid.trailingAnchor.constraint(lessThanOrEqualTo: content.trailingAnchor, constant: -20),
            grid.topAnchor.constraint(equalTo: content.topAnchor, constant: 20),
            grid.bottomAnchor.constraint(lessThanOrEqualTo: content.bottomAnchor, constant: -20),
        ])
        w.contentView = content
        return w
    }

    private func refresh() {
        modePopup.selectItem(at: Preferences.Mode.allCases.firstIndex(of: Preferences.mode) ?? 0)
        abbreviations.state = Preferences.abbreviations ? .on : .off
        keyUp.state = Preferences.useKeyUp ? .on : .off
        timeout.integerValue = Preferences.timeoutMs
        timeoutLabel.stringValue = "\(Preferences.timeoutMs) ms"
        let running = KeyTap.shared.isRunning
        tapStatus.stringValue = running ? "키뗌 감지: 동작 중" : "키뗌 감지: 입력 모니터링 권한이 없어 타이머로만 확정합니다"
        permission.isHidden = running
    }

    @objc private func modeChanged() {
        if let raw = modePopup.selectedItem?.representedObject as? String, let mode = Preferences.Mode(rawValue: raw) {
            Preferences.mode = mode
        }
    }

    @objc private func abbreviationsChanged() {
        Preferences.abbreviations = abbreviations.state == .on
    }

    @objc private func keyUpChanged() {
        Preferences.useKeyUp = keyUp.state == .on
    }

    @objc private func timeoutChanged() {
        Preferences.timeoutMs = timeout.integerValue
        timeoutLabel.stringValue = "\(Preferences.timeoutMs) ms"
    }

    @objc private func requestPermission() {
        KeyTap.shared.requestPermission()
        if KeyTap.shared.start() {
            NotificationCenter.default.post(name: Preferences.didChange, object: nil)
        }
        refresh()
    }
}

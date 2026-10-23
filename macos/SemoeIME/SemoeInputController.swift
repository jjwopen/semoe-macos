import AppKit
import InputMethodKit

/// Info.plist의 InputMethodServerControllerClass와 같은 이름으로 Objective-C에 노출한다.
/// 앱의 입력 세션마다 하나씩 생긴다. 입력 판단은 엔진(semoe-core)이 하고, 여기서는 이벤트를 모아
/// 엔진에 넣고 결과를 앱에 반영한다.
@objc(SemoeInputController)
final class SemoeInputController: IMKInputController {
    private let engine = EngineHandle()
    private var bridge = ClientBridge()
    private lazy var timer = CommitTimer { [weak self] deadline in self?.timerFired(deadline) }
    private var observer: NSObjectProtocol?
    /// 마지막으로 이벤트를 보낸 클라이언트. 키 이벤트 밖(탭 키뗌, 타이머)에서 글자를 넣을 때 쓴다.
    /// `client()`는 컨트롤러가 처음 받은 클라이언트라, 입력기를 다시 띄운 직후에는 실제로 키를 보내는
    /// 세션과 다를 수 있다. 그쪽으로 넣은 확정 문자열은 앱에 닿지 않았다 (Phase 3, Safari `ㅣㅇ` → `ㅣ`).
    private var currentClient: IMKTextInput?

    private var activeClient: IMKTextInput? {
        currentClient ?? client()
    }

    override init!(server: IMKServer!, delegate: Any!, client inputClient: Any!) {
        super.init(server: server, delegate: delegate, client: inputClient)
        observer = NotificationCenter.default.addObserver(
            forName: Preferences.didChange, object: nil, queue: .main
        ) { [weak self] _ in self?.preferencesChanged() }
    }

    deinit {
        if let observer { NotificationCenter.default.removeObserver(observer) }
    }

    // MARK: - 이벤트

    override func recognizedEvents(_ sender: Any!) -> Int {
        let mask: NSEvent.EventTypeMask = [.keyDown, .keyUp, .flagsChanged, .leftMouseDown, .rightMouseDown, .otherMouseDown]
        return Int(mask.rawValue)
    }

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, let client = sender as? IMKTextInput else { return false }
        currentClient = client
        let bundleID = client.bundleIdentifier() ?? "?"
        let tap = KeyTap.shared.isRunning
        switch event.type {
        case .keyDown where tap:
            // 키를 실제로 받는 컨트롤러가 탭 이벤트도 받는다. 활성화 순서로 정하면, 다른 세션(예: 시스템
            // 키보드 설정 확장)이 뒤늦게 활성화될 때 키뗌이 엉뚱한 세션으로 간다 (Phase 3에서 확인).
            KeyTap.shared.target = self
            // 탭 기록에서 이 keyDown의 짝을 찾아 그 앞의 물리 이벤트(주로 keyUp)부터 순서대로 넣는다.
            let (before, match) = KeyTap.shared.take(matching: event.keyCode)
            feedTap(before, client: client)
            if let match { EventLog.shared.key(match, bundleID: bundleID, consumed: true, kind: "tapdown") }
            let out = feed(event, up: false, timestamp: match?.timestamp ?? event.timestamp, client: client)
            EventLog.shared.key(event, bundleID: bundleID, consumed: out.consumed,
                                kind: match == nil ? "down-nomatch" : "down", output: out)
            return out.consumed
        case .keyUp where tap:
            // 키뗌은 탭 기록에서 받는다.
            EventLog.shared.key(event, bundleID: bundleID, consumed: false)
            return false
        case .keyDown, .keyUp:
            let out = feed(event, up: event.type == .keyUp, timestamp: event.timestamp, client: client)
            EventLog.shared.key(event, bundleID: bundleID, consumed: out.consumed, output: out)
            return out.consumed
        case .leftMouseDown, .rightMouseDown, .otherMouseDown:
            // 커서를 옮기기 전에 조합을 확정한다. 이벤트는 앱에 그대로 넘긴다.
            if bridge.hasMarked {
                EventLog.shared.note("mouse", bundleID: bundleID)
                apply(engine.flush(), to: client)
            }
            return false
        default:
            EventLog.shared.key(event, bundleID: bundleID, consumed: false)
            return false
        }
    }

    /// 탭 스레드가 새 이벤트를 기록할 때마다 불린다. 짝을 기다리는 keyDown 앞까지만 처리한다.
    func drainKeyTap() {
        let events = KeyTap.shared.takeReady()
        guard !events.isEmpty, let client = activeClient else { return }
        feedTap(events, client: client)
    }

    /// 탭 이벤트 중 keyUp만 엔진에 넣는다. IMK 짝이 없는 keyDown은 입력기로 오지 않은 키이므로 버린다.
    private func feedTap(_ events: [NSEvent], client: IMKTextInput) {
        let bundleID = client.bundleIdentifier() ?? "?"
        for e in events {
            if e.type == .keyUp {
                let out = feed(e, up: true, timestamp: e.timestamp, client: client)
                EventLog.shared.key(e, bundleID: bundleID, consumed: out.consumed, kind: "tapup", output: out)
            } else {
                EventLog.shared.key(e, bundleID: bundleID, consumed: false, kind: "tapskip")
            }
        }
    }

    @discardableResult
    private func feed(_ event: NSEvent, up: Bool, timestamp: TimeInterval, client: IMKTextInput) -> EngineHandle.Output {
        let out = engine.handleKey(
            keyCode: event.keyCode,
            up: up,
            flags: event.modifierFlags,
            isRepeat: event.isARepeat,
            timeMs: UInt64(timestamp * 1000)
        )
        apply(out, to: client)
        return out
    }

    // MARK: - 세션

    override func activateServer(_ sender: Any!) {
        super.activateServer(sender)
        currentClient = sender as? IMKTextInput
        KeyTap.shared.target = self
        KeyTap.shared.start()
        EventLog.shared.note("activate", bundleID: Self.bundleID(sender), Preferences.apply(to: engine))
    }

    override func deactivateServer(_ sender: Any!) {
        flush(sender)
        if KeyTap.shared.target === self { KeyTap.shared.target = nil }
        EventLog.shared.note("deactivate", bundleID: Self.bundleID(sender))
        currentClient = nil
        super.deactivateServer(sender)
    }

    override func commitComposition(_ sender: Any!) {
        EventLog.shared.note("commitComposition", bundleID: Self.bundleID(sender), bridge.hasMarked ? "marked" : "")
        flush(sender)
    }

    private func flush(_ sender: Any!) {
        let out = engine.flush()
        if let client = sender as? IMKTextInput {
            apply(out, to: client)
        } else {
            timer.cancel()
        }
    }

    private func preferencesChanged() {
        // 입력 방식을 바꾸면 엔진이 조합을 버리므로 먼저 확정한다.
        if let client = activeClient {
            apply(engine.flush(), to: client)
        }
        EventLog.shared.note("option", bundleID: "-", Preferences.apply(to: engine))
    }

    // MARK: - 출력

    private func apply(_ out: EngineHandle.Output, to client: IMKTextInput) {
        bridge.apply(out, to: client)
        timer.schedule(out.deadlineMs)
    }

    private func timerFired(_ deadlineMs: UInt64) {
        guard let client = activeClient else { return timer.cancel() }
        // 마감 전에 눌렸지만 IMK가 아직 넘기지 않은 keyDown이 있으면 그 키도 이 음절에 속한다.
        // 앱이 keyDown을 수십 ms 늦게 넘기는 경우가 있어(Phase 0), 도착할 때까지 확정을 미룬다.
        if let pending = KeyTap.shared.oldestPendingDownMs(), pending < Double(deadlineMs) {
            EventLog.shared.note("timer-wait", bundleID: client.bundleIdentifier() ?? "?")
            timer.schedule(deadlineMs, afterMs: 2)
            return
        }
        EventLog.shared.timer(bundleID: client.bundleIdentifier() ?? "?", deadlineMs: deadlineMs)
        // 마감 직전에 발화한 경우를 막으려고 올림한다.
        apply(engine.onTimer(nowMs: UInt64(EventLog.nowMs().rounded(.up))), to: client)
    }

    private static func bundleID(_ sender: Any!) -> String {
        (sender as? IMKTextInput)?.bundleIdentifier() ?? "?"
    }

    // MARK: - 메뉴

    override func menu() -> NSMenu! {
        let menu = NSMenu()
        for (mode, action) in [(Preferences.Mode.chord, #selector(useChord(_:))),
                               (.sequential, #selector(useSequential(_:)))] {
            let item = NSMenuItem(title: mode.title, action: action, keyEquivalent: "")
            item.state = Preferences.mode == mode ? .on : .off
            menu.addItem(item)
        }
        menu.addItem(.separator())
        for (title, on, action) in [
            ("약어 사용", Preferences.abbreviations, #selector(toggleAbbreviations(_:))),
            ("기호 조합 사용", Preferences.compose, #selector(toggleCompose(_:))),
            ("옛한글 모드", Preferences.oldHangul, #selector(toggleOldHangul(_:))),
        ] {
            let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
            item.state = on ? .on : .off
            menu.addItem(item)
        }
        if !KeyTap.shared.isRunning {
            menu.addItem(NSMenuItem(title: "키뗌 감지 꺼짐 (입력 모니터링 권한 필요)", action: nil, keyEquivalent: ""))
        }
        menu.addItem(NSMenuItem(title: "환경설정…", action: #selector(showPreferences(_:)), keyEquivalent: ""))
        menu.addItem(.separator())
        let log = NSMenuItem(title: "이벤트 로그 기록 (개발용)", action: #selector(toggleLog(_:)), keyEquivalent: "")
        log.state = EventLog.shared.enabled ? .on : .off
        menu.addItem(log)
        menu.addItem(NSMenuItem(title: "로그 폴더 열기", action: #selector(openLog(_:)), keyEquivalent: ""))
        return menu
    }

    @objc func useChord(_ sender: Any?) {
        Preferences.mode = .chord
    }

    @objc func useSequential(_ sender: Any?) {
        Preferences.mode = .sequential
    }

    @objc func toggleAbbreviations(_ sender: Any?) {
        Preferences.abbreviations.toggle()
    }

    @objc func toggleCompose(_ sender: Any?) {
        Preferences.compose.toggle()
    }

    @objc func toggleOldHangul(_ sender: Any?) {
        Preferences.oldHangul.toggle()
    }

    override func showPreferences(_ sender: Any!) {
        PreferencesWindow.shared.show()
    }

    @objc func toggleLog(_ sender: Any?) {
        EventLog.shared.enabled.toggle()
    }

    @objc func openLog(_ sender: Any?) {
        try? FileManager.default.createDirectory(at: EventLog.shared.directory, withIntermediateDirectories: true)
        NSWorkspace.shared.open(EventLog.shared.directory)
    }
}

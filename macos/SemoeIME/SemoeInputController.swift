import AppKit
import InputMethodKit

/// Info.plist의 InputMethodServerControllerClass와 같은 이름으로 Objective-C에 노출한다.
@objc(SemoeInputController)
final class SemoeInputController: IMKInputController {
    static let useKeyUpKey = "useKeyUp"

    private let engine = EngineHandle()
    private var timer: DispatchSourceTimer?
    private var hasMarked = false

    private static let notFound = NSRange(location: NSNotFound, length: NSNotFound)

    // MARK: - 이벤트

    override func recognizedEvents(_ sender: Any!) -> Int {
        Int(NSEvent.EventTypeMask([.keyDown, .keyUp, .flagsChanged]).rawValue)
    }

    override func handle(_ event: NSEvent!, client sender: Any!) -> Bool {
        guard let event, let client = sender as? IMKTextInput else { return false }
        let bundleID = client.bundleIdentifier() ?? "?"
        let tap = KeyTap.shared.isRunning
        switch event.type {
        case .keyDown where tap:
            // 탭 기록에서 이 keyDown의 짝을 찾아 그 앞의 물리 이벤트(주로 keyUp)부터 순서대로 넣는다.
            let (before, match) = KeyTap.shared.take(matching: event.keyCode)
            feedTap(before, client: client)
            if let match { EventLog.shared.key(match, bundleID: bundleID, consumed: true, kind: "tapdown") }
            let out = feed(event, up: false, timestamp: match?.timestamp ?? event.timestamp, client: client)
            EventLog.shared.key(event, bundleID: bundleID, consumed: out.consumed, kind: match == nil ? "down-nomatch" : "down")
            return out.consumed
        case .keyUp where tap:
            // 키뗌은 탭 기록에서 받는다.
            EventLog.shared.key(event, bundleID: bundleID, consumed: false)
            return false
        case .keyDown, .keyUp:
            let out = feed(event, up: event.type == .keyUp, timestamp: event.timestamp, client: client)
            EventLog.shared.key(event, bundleID: bundleID, consumed: out.consumed)
            return out.consumed
        default:
            EventLog.shared.key(event, bundleID: bundleID, consumed: false)
            return false
        }
    }

    /// 탭 스레드가 새 이벤트를 기록할 때마다 불린다. 짝을 기다리는 keyDown 앞까지만 처리한다.
    func drainKeyTap() {
        let events = KeyTap.shared.takeReady()
        guard !events.isEmpty, let client = client() else { return }
        feedTap(events, client: client)
    }

    /// 탭 이벤트 중 keyUp만 엔진에 넣는다. IMK 짝이 없는 keyDown은 입력기로 오지 않은 키이므로 버린다.
    private func feedTap(_ events: [NSEvent], client: IMKTextInput) {
        let bundleID = client.bundleIdentifier() ?? "?"
        for e in events {
            if e.type == .keyUp {
                let out = feed(e, up: true, timestamp: e.timestamp, client: client)
                EventLog.shared.key(e, bundleID: bundleID, consumed: out.consumed, kind: "tapup")
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
        KeyTap.shared.target = self
        KeyTap.shared.start()
        EventLog.shared.note("activate", bundleID: Self.bundleID(sender), applyKeyUpOption())
    }

    /// 키뗌 확정은 사용자가 켜 두었고 키 탭이 돌고 있을 때만 쓴다. 아니면 타이머 전용(방안 C).
    @discardableResult
    private func applyKeyUpOption() -> String {
        let wanted = UserDefaults.standard.object(forKey: Self.useKeyUpKey) as? Bool ?? true
        let tap = KeyTap.shared.isRunning
        engine.setOption("keyup", wanted && tap ? "on" : "off")
        return "keyup=\(wanted ? "on" : "off") tap=\(tap ? "on" : "off")"
    }

    override func deactivateServer(_ sender: Any!) {
        flush(sender)
        if KeyTap.shared.target === self { KeyTap.shared.target = nil }
        EventLog.shared.note("deactivate", bundleID: Self.bundleID(sender))
        super.deactivateServer(sender)
    }

    override func commitComposition(_ sender: Any!) {
        EventLog.shared.note("commitComposition", bundleID: Self.bundleID(sender), hasMarked ? "marked" : "")
        flush(sender)
    }

    private func flush(_ sender: Any!) {
        let out = engine.flush()
        if let client = sender as? IMKTextInput {
            apply(out, to: client)
        } else {
            cancelTimer()
        }
    }

    // MARK: - 출력

    private func apply(_ out: EngineHandle.Output, to client: IMKTextInput) {
        if !out.commit.isEmpty {
            client.insertText(out.commit, replacementRange: Self.notFound)
            hasMarked = false
        }
        if !out.marked.isEmpty {
            let length = (out.marked as NSString).length
            client.setMarkedText(out.marked, selectionRange: NSRange(location: length, length: 0),
                                 replacementRange: Self.notFound)
            hasMarked = true
        } else if hasMarked {
            client.setMarkedText("", selectionRange: NSRange(location: 0, length: 0), replacementRange: Self.notFound)
            hasMarked = false
        }
        schedule(out.deadlineMs)
    }

    private func schedule(_ deadlineMs: UInt64?) {
        cancelTimer()
        guard let deadlineMs else { return }
        let delay = max(0, Double(deadlineMs) - EventLog.nowMs())
        let t = DispatchSource.makeTimerSource(flags: .strict, queue: .main)
        t.schedule(deadline: .now() + .microseconds(Int(delay * 1000)), leeway: .milliseconds(1))
        t.setEventHandler { [weak self] in self?.timerFired(deadlineMs) }
        t.resume()
        timer = t
    }

    private func cancelTimer() {
        timer?.cancel()
        timer = nil
    }

    private func timerFired(_ deadlineMs: UInt64) {
        guard let client = client() else { return cancelTimer() }
        // 마감 전에 눌렸지만 IMK가 아직 넘기지 않은 keyDown이 있으면 그 키도 이 음절에 속한다.
        // 앱이 keyDown을 수십 ms 늦게 넘기는 경우가 있어(Phase 0), 도착할 때까지 확정을 미룬다.
        if let pending = KeyTap.shared.oldestPendingDownMs(), pending < Double(deadlineMs) {
            EventLog.shared.note("timer-wait", bundleID: client.bundleIdentifier() ?? "?")
            let t = DispatchSource.makeTimerSource(flags: .strict, queue: .main)
            t.schedule(deadline: .now() + .milliseconds(2), leeway: .milliseconds(1))
            t.setEventHandler { [weak self] in self?.timerFired(deadlineMs) }
            t.resume()
            cancelTimer()
            timer = t
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
        let useKeyUp = UserDefaults.standard.object(forKey: Self.useKeyUpKey) as? Bool ?? true
        let keyUp = NSMenuItem(title: "키뗌으로 확정 (끄면 타이머 전용)", action: #selector(toggleKeyUp(_:)), keyEquivalent: "")
        keyUp.state = useKeyUp ? .on : .off
        menu.addItem(keyUp)
        if KeyTap.shared.isRunning {
            menu.addItem(NSMenuItem(title: "키뗌 감지: 동작 중", action: nil, keyEquivalent: ""))
        } else {
            menu.addItem(NSMenuItem(title: "키뗌 감지: 입력 모니터링 권한 없음 (타이머 전용)", action: nil, keyEquivalent: ""))
            menu.addItem(NSMenuItem(title: "입력 모니터링 권한 요청…", action: #selector(requestTapPermission(_:)), keyEquivalent: ""))
        }
        menu.addItem(.separator())
        let log = NSMenuItem(title: "Phase 0 이벤트 로그 기록", action: #selector(toggleLog(_:)), keyEquivalent: "")
        log.state = EventLog.shared.enabled ? .on : .off
        menu.addItem(log)
        menu.addItem(NSMenuItem(title: "로그 폴더 열기", action: #selector(openLog(_:)), keyEquivalent: ""))
        return menu
    }

    @objc func toggleKeyUp(_ sender: Any?) {
        let useKeyUp = !(UserDefaults.standard.object(forKey: Self.useKeyUpKey) as? Bool ?? true)
        UserDefaults.standard.set(useKeyUp, forKey: Self.useKeyUpKey)
        EventLog.shared.note("option", bundleID: "-", applyKeyUpOption())
    }

    @objc func requestTapPermission(_ sender: Any?) {
        KeyTap.shared.requestPermission()
        if KeyTap.shared.start() {
            applyKeyUpOption()
        }
    }

    @objc func toggleLog(_ sender: Any?) {
        EventLog.shared.enabled.toggle()
    }

    @objc func openLog(_ sender: Any?) {
        try? FileManager.default.createDirectory(at: EventLog.shared.directory, withIntermediateDirectories: true)
        NSWorkspace.shared.open(EventLog.shared.directory)
    }
}

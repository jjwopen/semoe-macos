import AppKit

/// 물리 키 이벤트 스트림 (계획서 3.1 방안 B).
///
/// IMK는 keyUp을 넘기지 않고, keyDown도 앱이 붙잡았다가 넘기므로 시각과 간격이 실제와 다르다(Phase 0 검증).
/// 그래서 세션 이벤트 탭(listenOnly)을 별도 스레드에서 돌려 keyDown·keyUp을 물리 순서와 하드웨어 시각
/// 그대로 기록하고, 컨트롤러가 IMK keyDown을 받을 때 이 기록과 맞춰 엔진에 순서대로 넣는다.
///
/// "입력 모니터링" 권한이 필요하다. 권한이 없으면 `isRunning == false`이고 엔진은 타이머 전용으로 동작한다.
final class KeyTap {
    static let shared = KeyTap()

    /// IMK keyDown과 짝이 맞지 않은 탭 keyDown을 버리기까지의 시간. 입력기로 오지 않는 키(다른 입력 소스,
    /// 텍스트 입력이 아닌 곳)가 기록 맨 앞에서 뒤 이벤트를 막지 않게 한다.
    static let unmatchedDownTimeoutMs = 200.0

    /// IMK keyDown이 왔는데 탭 기록에 아직 짝이 없을 때 기다리는 최대 시간.
    static let matchWaitMs = 10.0

    /// 지금 활성화된 컨트롤러. 탭은 프로세스에 하나뿐이고 이벤트는 이 컨트롤러로만 보낸다.
    weak var target: SemoeInputController?

    private let lock = NSCondition()
    private var buffer: [CGEvent] = []
    private var tap: CFMachPort?
    private var thread: Thread?

    var isRunning: Bool { lock.sync { tap }.map { CGEvent.tapIsEnabled(tap: $0) } ?? false }

    static var hasPermission: Bool { CGPreflightListenEventAccess() }

    /// 권한이 있으면 탭 스레드를 띄운다. 이미 돌고 있으면 아무것도 하지 않는다.
    @discardableResult
    func start() -> Bool {
        if isRunning { return true }
        guard Self.hasPermission, thread == nil else { return false }
        let ready = DispatchSemaphore(value: 0)
        let t = Thread { [self] in
            let mask = CGEventMask((1 << CGEventType.keyDown.rawValue) | (1 << CGEventType.keyUp.rawValue))
            guard let port = CGEvent.tapCreate(
                tap: .cgSessionEventTap, place: .headInsertEventTap, options: .listenOnly,
                eventsOfInterest: mask, callback: keyTapCallback, userInfo: nil
            ) else {
                ready.signal()
                return
            }
            CFRunLoopAddSource(CFRunLoopGetCurrent(), CFMachPortCreateRunLoopSource(nil, port, 0), .commonModes)
            CGEvent.tapEnable(tap: port, enable: true)
            lock.sync { tap = port }
            ready.signal()
            CFRunLoopRun()
        }
        t.name = "SemoeKeyTap"
        t.qualityOfService = .userInteractive
        thread = t
        t.start()
        ready.wait()
        if !isRunning { thread = nil }
        return isRunning
    }

    /// 시스템 권한 요청 창을 띄운다.
    func requestPermission() {
        CGRequestListenEventAccess()
    }

    // MARK: - 탭 스레드

    fileprivate func record(_ type: CGEventType, _ event: CGEvent) {
        switch type {
        case .tapDisabledByTimeout, .tapDisabledByUserInput:
            if let tap = lock.sync({ tap }) { CGEvent.tapEnable(tap: tap, enable: true) }
        case .keyDown, .keyUp:
            lock.sync {
                // listenOnly 탭이 넘기는 이벤트는 재사용될 수 있으므로 복사해 둔다.
                buffer.append(event.copy() ?? event)
                lock.broadcast()
            }
            DispatchQueue.main.async { [weak self] in self?.target?.drainKeyTap() }
        default:
            break
        }
    }

    // MARK: - 메인 스레드

    /// IMK keyDown에 맞는 탭 keyDown을 찾아, 그 앞 이벤트들과 함께 꺼낸다.
    /// IMK는 물리 순서대로 keyDown을 넘기므로 짝 앞의 짝 없는 keyDown은 입력기로 오지 않은 키다.
    /// 짝을 `matchWaitMs` 안에 못 찾으면 `match == nil`.
    func take(matching keyCode: UInt16) -> (before: [NSEvent], match: NSEvent?) {
        let deadline = Date().addingTimeInterval(Self.matchWaitMs / 1000)
        let events: [CGEvent]? = lock.sync {
            repeat {
                if let i = buffer.firstIndex(where: {
                    $0.type == .keyDown && UInt16($0.getIntegerValueField(.keyboardEventKeycode)) == keyCode
                }) {
                    let taken = Array(buffer[...i])
                    buffer.removeFirst(i + 1)
                    return taken
                }
            } while lock.wait(until: deadline)
            return nil
        }
        guard let events else { return ([], nil) }
        let ns = events.compactMap(NSEvent.init(cgEvent:))
        return (Array(ns.dropLast()), ns.last)
    }

    /// IMK 짝을 기다리는 가장 이른 keyDown의 물리 시각(ms). 버릴 만큼 오래된 것은 무시한다.
    func oldestPendingDownMs() -> Double? {
        let nowMs = EventLog.nowMs()
        return lock.sync {
            buffer.lazy
                .filter { $0.type == .keyDown }
                .compactMap { NSEvent(cgEvent: $0).map { $0.timestamp * 1000 } }
                .first { nowMs - $0 < Self.unmatchedDownTimeoutMs }
        }
    }

    /// 지금 처리해도 되는 이벤트를 앞에서부터 꺼낸다: keyUp, 그리고 오래되어 버리는 짝 없는 keyDown.
    /// 아직 IMK 짝을 기다리는 keyDown을 만나면 멈춘다. 그 뒤의 keyUp을 먼저 처리하면 chord가 일찍 끝난다.
    func takeReady() -> [NSEvent] {
        let nowMs = EventLog.nowMs()
        let events: [CGEvent] = lock.sync {
            var n = 0
            for e in buffer {
                if e.type == .keyDown,
                   let ns = NSEvent(cgEvent: e), nowMs - ns.timestamp * 1000 < Self.unmatchedDownTimeoutMs {
                    break
                }
                n += 1
            }
            let taken = Array(buffer.prefix(n))
            buffer.removeFirst(n)
            return taken
        }
        return events.compactMap(NSEvent.init(cgEvent:))
    }
}

private func keyTapCallback(
    proxy: CGEventTapProxy, type: CGEventType, event: CGEvent, userInfo: UnsafeMutableRawPointer?
) -> Unmanaged<CGEvent>? {
    KeyTap.shared.record(type, event)
    return Unmanaged.passUnretained(event)
}

private extension NSCondition {
    func sync<T>(_ body: () throws -> T) rethrows -> T {
        lock()
        defer { unlock() }
        return try body()
    }
}

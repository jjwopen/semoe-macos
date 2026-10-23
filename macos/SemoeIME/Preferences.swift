import Foundation

/// 사용자 설정 (`UserDefaults`). 바뀌면 `didChange` 알림을 보내고, 컨트롤러마다 엔진에 반영한다.
enum Preferences {
    static let didChange = Notification.Name("SemoePreferencesDidChange")

    enum Mode: String, CaseIterable {
        /// 한 번에 모아치기
        case chord
        /// 이어치기·중종성 모아치기 (원본에서 같은 오토마타)
        case sequential

        var title: String {
            switch self {
            case .chord: "한 번에 모아치기"
            case .sequential: "이어치기 · 중종성 모아치기"
            }
        }
    }

    /// 조합 종료 타이머 범위 (원본 개발 규격: 20~100 ms, 기본 40 ms)
    static let timeoutRange = 20...100
    static let timeoutStep = 5

    private enum Key {
        static let mode = "mode"
        static let timeoutMs = "timeoutMs"
        static let abbreviations = "abbreviations"
        static let useKeyUp = "useKeyUp"
        static let compose = "compose"
        static let oldHangul = "oldHangul"
        static let capsLockShift = "capsLockShift"
    }

    private static var defaults: UserDefaults { .standard }

    static var mode: Mode {
        get { defaults.string(forKey: Key.mode).flatMap(Mode.init(rawValue:)) ?? .chord }
        set { set(newValue.rawValue, Key.mode) }
    }

    static var timeoutMs: Int {
        get {
            let v = defaults.object(forKey: Key.timeoutMs) as? Int ?? 40
            return min(max(v, timeoutRange.lowerBound), timeoutRange.upperBound)
        }
        set {
            let stepped = Int((Double(newValue) / Double(timeoutStep)).rounded()) * timeoutStep
            set(min(max(stepped, timeoutRange.lowerBound), timeoutRange.upperBound), Key.timeoutMs)
        }
    }

    static var abbreviations: Bool {
        get { defaults.object(forKey: Key.abbreviations) as? Bool ?? true }
        set { set(newValue, Key.abbreviations) }
    }

    /// 키뗌으로 확정 (끄면 타이머 전용, 계획서 3.1 방안 C)
    static var useKeyUp: Bool {
        get { defaults.object(forKey: Key.useKeyUp) as? Bool ?? true }
        set { set(newValue, Key.useKeyUp) }
    }

    /// 기호 조합: 기호 두 개를 이어 치면 정해진 문자열이 된다 (※ △ → 가? )
    static var compose: Bool {
        get { defaults.object(forKey: Key.compose) as? Bool ?? true }
        set { set(newValue, Key.compose) }
    }

    /// 옛한글 모드: 숫자열 옛한글, 약어 끔, Shift+Y·U 방점 (원본의 Scroll Lock)
    static var oldHangul: Bool {
        get { defaults.object(forKey: Key.oldHangul) as? Bool ?? false }
        set { set(newValue, Key.oldHangul) }
    }

    /// Caps Lock 순아래(Shift Lock). Caps Lock을 한/영 전환에 쓰고 있으면 겹친다.
    static var capsLockShift: Bool {
        get { defaults.object(forKey: Key.capsLockShift) as? Bool ?? false }
        set { set(newValue, Key.capsLockShift) }
    }

    private static func set(_ value: Any, _ key: String) {
        defaults.set(value, forKey: key)
        NotificationCenter.default.post(name: didChange, object: nil)
    }

    /// 엔진에 반영한다. 반영한 내용을 로그용 문자열로 돌려준다.
    @discardableResult
    static func apply(to engine: EngineHandle) -> String {
        let tap = KeyTap.shared.isRunning
        // 키뗌은 탭이 돌고 있을 때만 믿을 수 있다.
        let keyUp = useKeyUp && tap
        engine.setOption("mode", mode.rawValue)
        engine.setOption("timeout_ms", String(timeoutMs))
        engine.setOption("abbrev", abbreviations ? "on" : "off")
        engine.setOption("keyup", keyUp ? "on" : "off")
        engine.setOption("compose", compose ? "on" : "off")
        engine.setOption("old_hangul", oldHangul ? "on" : "off")
        engine.setOption("caps_lock_shift", capsLockShift ? "on" : "off")
        func onOff(_ v: Bool) -> String { v ? "on" : "off" }
        return "mode=\(mode.rawValue) timeout=\(timeoutMs) abbrev=\(onOff(abbreviations)) compose=\(onOff(compose)) "
            + "old=\(onOff(oldHangul)) caps=\(onOff(capsLockShift)) keyup=\(onOff(useKeyUp)) tap=\(onOff(tap))"
    }
}

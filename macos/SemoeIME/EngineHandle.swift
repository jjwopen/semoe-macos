import AppKit
import SemoeFFI

/// Rust 엔진 래퍼. 컨트롤러마다 하나씩 둔다. 메인 스레드에서만 쓴다.
final class EngineHandle {
    struct Output {
        var consumed: Bool
        var commit: String
        var marked: String
        /// 이 시각(ms)에 onTimer를 불러야 한다. nil이면 타이머 해제.
        var deadlineMs: UInt64?
    }

    private let raw: OpaquePointer?

    init() {
        raw = semoe_engine_new()
    }

    deinit {
        semoe_engine_free(raw)
    }

    func handleKey(keyCode: UInt16, up: Bool, flags: NSEvent.ModifierFlags, isRepeat: Bool, timeMs: UInt64) -> Output {
        let ev = SemoeKeyEvent(
            key: KeyMapper.key(for: keyCode),
            action: up ? SEMOE_KEY_UP : SEMOE_KEY_DOWN,
            modifiers: KeyMapper.modifiers(flags),
            is_repeat: isRepeat,
            time_ms: timeMs
        )
        return Self.copy(semoe_handle_key(raw, ev))
    }

    func onTimer(nowMs: UInt64) -> Output {
        Self.copy(semoe_on_timer(raw, nowMs))
    }

    func flush() -> Output {
        Self.copy(semoe_flush(raw))
    }

    @discardableResult
    func setOption(_ key: String, _ value: String) -> Bool {
        semoe_set_option(raw, key, value)
    }

    /// 엔진 소유 문자열은 다음 호출 전까지만 유효하므로 즉시 복사한다.
    private static func copy(_ out: SemoeOutput) -> Output {
        Output(
            consumed: out.consumed,
            commit: out.commit.map { String(cString: $0) } ?? "",
            marked: out.marked.map { String(cString: $0) } ?? "",
            deadlineMs: out.deadline_ms >= 0 ? UInt64(out.deadline_ms) : nil
        )
    }
}

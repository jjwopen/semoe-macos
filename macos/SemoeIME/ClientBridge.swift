import AppKit
import InputMethodKit

/// 엔진 출력을 앱(클라이언트)에 반영한다: 확정 문자열 삽입, 조합 중 문자열 표시.
struct ClientBridge {
    private static let notFound = NSRange(location: NSNotFound, length: NSNotFound)

    /// 앱에 조합 중 문자열을 보여 주고 있는가
    private(set) var hasMarked = false

    /// 커서 앞 문자열을 바꿀 수 있는 클라이언트인가. 엔진의 `compose_inline`을 정하는 데 쓴다.
    static func canReplace(_ client: IMKTextInput) -> Bool {
        let sel = client.selectedRange()
        return sel.location != NSNotFound && sel.length != NSNotFound
    }

    mutating func apply(_ out: EngineHandle.Output, to client: IMKTextInput) {
        if !out.commit.isEmpty || !out.replace.isEmpty {
            client.insertText(out.commit, replacementRange: Self.range(before: out.replace, in: client))
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
    }

    /// 커서 바로 앞의 `text` 범위. 커서 앞이 `text`가 아니면 바꾸지 않고 그냥 넣는다.
    private static func range(before text: String, in client: IMKTextInput) -> NSRange {
        let length = (text as NSString).length
        guard length > 0 else { return notFound }
        let sel = client.selectedRange()
        guard sel.location != NSNotFound, sel.length == 0, sel.location >= length else { return notFound }
        let range = NSRange(location: sel.location - length, length: length)
        guard client.attributedSubstring(from: range)?.string == text else { return notFound }
        return range
    }
}

/// 엔진이 알려 준 마감 시각(ms, `ProcessInfo.systemUptime` 기준)에 한 번 울리는 타이머.
final class CommitTimer {
    private var source: DispatchSourceTimer?
    private let fire: (UInt64) -> Void

    init(fire: @escaping (UInt64) -> Void) {
        self.fire = fire
    }

    /// `deadlineMs`가 `nil`이면 해제한다. `afterMs`가 있으면 마감과 상관없이 그만큼 뒤에 울린다.
    func schedule(_ deadlineMs: UInt64?, afterMs: Double? = nil) {
        cancel()
        guard let deadlineMs else { return }
        let delay = afterMs ?? max(0, Double(deadlineMs) - EventLog.nowMs())
        let t = DispatchSource.makeTimerSource(flags: .strict, queue: .main)
        t.schedule(deadline: .now() + .microseconds(Int(delay * 1000)), leeway: .milliseconds(1))
        t.setEventHandler { [weak self] in self?.fire(deadlineMs) }
        t.resume()
        source = t
    }

    func cancel() {
        source?.cancel()
        source = nil
    }

    deinit {
        cancel()
    }
}

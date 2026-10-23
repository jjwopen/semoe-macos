import AppKit

/// Phase 0 검증용 이벤트 로그 (~/Library/Logs/SemoeIME/events.tsv).
///
/// 키 코드만 남기고 문자·조합 결과는 남기지 않는다. 그래도 키 코드로 입력 내용을 복원할 수 있으므로
/// 검증이 끝나면 메뉴에서 끄거나 파일을 지운다.
///
/// 열: uptime_ms  kind  bundle_id  key_code  repeat  event_ms  lag_ms  extra
/// - kind: down/up/down-nomatch(IMK), tapdown/tapup/tapskip(KeyTap), flags, timer, activate/deactivate/option
/// - lag_ms: 이벤트 발생 시각(NSEvent.timestamp)부터 입력기가 받은 시각까지
/// - timer 행의 extra: 마감 시각 대비 실제 발화 지연(ms)
/// - 키 행의 extra: consumed/pass와 엔진 출력의 글자 수(c=확정, m=조합 중)
final class EventLog {
    static let shared = EventLog()

    static let defaultsKey = "eventLogEnabled"

    var enabled: Bool {
        get { UserDefaults.standard.object(forKey: Self.defaultsKey) as? Bool ?? true }
        set { UserDefaults.standard.set(newValue, forKey: Self.defaultsKey) }
    }

    let directory = FileManager.default.homeDirectoryForCurrentUser
        .appendingPathComponent("Library/Logs/SemoeIME", isDirectory: true)

    private lazy var handle: FileHandle? = {
        let url = directory.appendingPathComponent("events.tsv")
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        if !FileManager.default.fileExists(atPath: url.path) {
            let header = "uptime_ms\tkind\tbundle_id\tkey_code\trepeat\tevent_ms\tlag_ms\textra\n"
            FileManager.default.createFile(atPath: url.path, contents: Data(header.utf8))
        }
        let h = try? FileHandle(forWritingTo: url)
        _ = try? h?.seekToEnd()
        return h
    }()

    static func nowMs() -> Double {
        ProcessInfo.processInfo.systemUptime * 1000
    }

    func key(_ event: NSEvent, bundleID: String, consumed: Bool, kind kindOverride: String? = nil,
             output: EngineHandle.Output? = nil) {
        let now = Self.nowMs()
        let eventMs = event.timestamp * 1000
        var kind: String
        var isRepeat = "-"
        switch event.type {
        case .keyDown: kind = "down"; isRepeat = event.isARepeat ? "1" : "0"
        case .keyUp: kind = "up"; isRepeat = event.isARepeat ? "1" : "0"
        case .flagsChanged: kind = "flags"
        default: kind = "other"
        }
        if let kindOverride { kind = kindOverride }
        write([fmt(now), kind, bundleID, String(event.keyCode), isRepeat, fmt(eventMs), fmt(now - eventMs),
               (consumed ? "consumed" : "pass") + (output.map(Self.shape) ?? "")])
    }

    /// 엔진 출력의 꼴: 확정·조합 문자열의 글자 수만 남긴다(내용은 남기지 않음).
    static func shape(_ out: EngineHandle.Output) -> String {
        " c=\(out.commit.count) m=\(out.marked.count)"
    }

    func timer(bundleID: String, deadlineMs: UInt64) {
        let now = Self.nowMs()
        write([fmt(now), "timer", bundleID, "-", "-", "-", "-", fmt(now - Double(deadlineMs))])
    }

    func note(_ kind: String, bundleID: String, _ extra: String = "") {
        write([fmt(Self.nowMs()), kind, bundleID, "-", "-", "-", "-", extra])
    }

    private func fmt(_ v: Double) -> String {
        String(format: "%.2f", v)
    }

    private func write(_ fields: [String]) {
        guard enabled else { return }
        handle?.write(Data((fields.joined(separator: "\t") + "\n").utf8))
    }
}

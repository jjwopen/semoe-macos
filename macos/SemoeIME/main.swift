import AppKit
import InputMethodKit

// 설치 스크립트용 명령. 입력기 서버를 띄우지 않고 끝난다.
switch CommandLine.arguments.dropFirst().first {
case "--register":
    exit(InputSourceRegistration.register() ? 0 : 1)
case "--unregister":
    exit(InputSourceRegistration.unregister() ? 0 : 1)
case "--version":
    print(Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "?")
    exit(0)
default:
    break
}

let connectionName = Bundle.main.object(forInfoDictionaryKey: "InputMethodConnectionName") as! String
// 프로세스가 끝날 때까지 살아 있어야 한다.
let server = IMKServer(name: connectionName, bundleIdentifier: Bundle.main.bundleIdentifier)

// 입력기 프로세스가 언제 새로 떴는지 로그로 추적한다 (Phase 0에서 연결이 끊긴 사례 조사용).
EventLog.shared.note("launch", bundleID: "-", "pid=\(ProcessInfo.processInfo.processIdentifier)")

NSApplication.shared.run()

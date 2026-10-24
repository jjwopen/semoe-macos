import Carbon
import Foundation

/// 입력 소스 등록·해제. 설치 스크립트가 `SemoeIME --register` / `--unregister`로 부른다
/// (사용자 Mac에는 Swift 도구가 없으므로 앱이 직접 한다).
///
/// 처음 설치한 입력기는 사용자가 시스템 설정에서 한 번 추가해야 선택할 수 있다(Phase 0 확인).
enum InputSourceRegistration {
    private static var bundleID: String { Bundle.main.bundleIdentifier ?? "" }

    /// 이 입력기의 입력 소스들 (번들 자체와 입력 모드)
    private static func sources() -> [TISInputSource] {
        let filter = [kTISPropertyBundleID as String: bundleID] as CFDictionary
        return (TISCreateInputSourceList(filter, true)?.takeRetainedValue() as? [TISInputSource]) ?? []
    }

    private static func report(_ status: OSStatus, _ what: String) -> Bool {
        if status != noErr {
            FileHandle.standardError.write(Data("\(what): 오류 \(status)\n".utf8))
        }
        return status == noErr
    }

    /// 앱 번들을 입력 소스로 등록하고 켠다. 성공하면 true.
    static func register() -> Bool {
        guard report(TISRegisterInputSource(Bundle.main.bundleURL as CFURL), "등록") else { return false }
        let found = sources()
        guard !found.isEmpty else {
            FileHandle.standardError.write(Data("등록했지만 입력 소스를 찾지 못함\n".utf8))
            return false
        }
        // 모드가 있는 입력기는 상위 소스(번들 ID)와 모드를 모두 켜야 한다(Phase 0 확인).
        return found.map { report(TISEnableInputSource($0), "켜기") }.allSatisfy { $0 }
    }

    /// 입력 소스를 끈다. 앱 파일은 설치 스크립트가 지운다.
    static func unregister() -> Bool {
        sources().map { report(TISDisableInputSource($0), "끄기") }.allSatisfy { $0 }
    }
}

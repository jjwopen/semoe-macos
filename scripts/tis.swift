// 입력 소스 등록·활성화 도우미. 사용:
//   swift scripts/tis.swift register <앱 경로>
//   swift scripts/tis.swift enable|disable|select <입력 소스 ID>
//   swift scripts/tis.swift status [ID 접두어]
import Carbon
import Foundation

func sources(_ id: String? = nil) -> [TISInputSource] {
    var filter: [String: Any] = [:]
    if let id { filter[kTISPropertyInputSourceID as String] = id }
    return (TISCreateInputSourceList(filter as CFDictionary, true)?.takeRetainedValue() as? [TISInputSource]) ?? []
}

func prop<T>(_ s: TISInputSource, _ key: CFString) -> T? {
    guard let p = TISGetInputSourceProperty(s, key) else { return nil }
    return Unmanaged<AnyObject>.fromOpaque(p).takeUnretainedValue() as? T
}

func check(_ status: OSStatus, _ what: String) {
    print("\(what): \(status == noErr ? "ok" : "error \(status)")")
    if status != noErr { exit(1) }
}

let args = CommandLine.arguments.dropFirst()
switch (args.first, args.dropFirst().first) {
case ("register", let path?):
    check(TISRegisterInputSource(URL(fileURLWithPath: path) as CFURL), "register \(path)")
case ("enable", let id?):
    for s in sources(id) { check(TISEnableInputSource(s), "enable \(id)") }
case ("disable", let id?):
    for s in sources(id) { check(TISDisableInputSource(s), "disable \(id)") }
case ("select", let id?):
    for s in sources(id) { check(TISSelectInputSource(s), "select \(id)") }
case ("status", let prefix):
    for s in sources() {
        let id: String = prop(s, kTISPropertyInputSourceID) ?? "?"
        guard id.hasPrefix(prefix ?? "io.github.jjwopen") else { continue }
        let enabled: Bool = prop(s, kTISPropertyInputSourceIsEnabled) ?? false
        let selected: Bool = prop(s, kTISPropertyInputSourceIsSelected) ?? false
        print("\(id)\tenabled=\(enabled)\tselected=\(selected)")
    }
default:
    print("usage: tis.swift register <path> | enable|disable|select <id> | status [prefix]")
    exit(2)
}

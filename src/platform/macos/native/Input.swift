import AppKit
import ApplicationServices

struct MouseConfig: Codable {
    var version = 1
    var enabled = false
    var back = "默认"
    var forward = "默认"
    var middle = "默认"
}

enum MouseAction {
    case original, disabled, shortcut(CGKeyCode, CGEventFlags)
    case system(SystemAction)
}

enum SystemAction: String, CaseIterable {
    case apps = "打开 App"
    case missionControl = "调度中心"

    var bundleIdentifiers: [String] {
        switch self {
        case .apps: return ["com.apple.apps.launcher", "com.apple.launchpad.launcher"]
        case .missionControl: return ["com.apple.exposelauncher"]
        }
    }

    func applicationURL(in workspace: NSWorkspace = .shared) -> URL? {
        bundleIdentifiers.lazy.compactMap { workspace.urlForApplication(withBundleIdentifier:$0) }.first
    }

    func open(shouldOpen: @escaping () -> Bool = { true }, onFailure: @escaping (String) -> Void) {
        // Leave the event-tap callback promptly; Launch Services work belongs on main.
        DispatchQueue.main.async {
            guard shouldOpen() else { return }
            guard let url = self.applicationURL() else {
                onFailure("找不到系统功能：\(self.rawValue)")
                return
            }
            let options = NSWorkspace.OpenConfiguration()
            options.activates = true
            NSWorkspace.shared.openApplication(at:url, configuration:options) { _, error in
                if let error {
                    DispatchQueue.main.async { onFailure("无法打开\(self.rawValue)：\(error.localizedDescription)") }
                }
            }
        }
    }
}

enum ShortcutParser {
    static let keys: [String: CGKeyCode] = [
        "A":0, "S":1, "D":2, "F":3, "H":4, "G":5, "Z":6, "X":7,
        "C":8, "V":9, "B":11, "Q":12, "W":13, "E":14, "R":15,
        "Y":16, "T":17, "1":18, "2":19, "3":20, "4":21, "6":22,
        "5":23, "9":25, "7":26, "8":28, "0":29, "O":31, "U":32,
        "I":34, "P":35, "ENTER":36, "L":37, "J":38, "K":40, "N":45,
        "M":46, "TAB":48, "SPACE":49, "DELETE":117, "ESC":53,
        "F17":64, "F18":79, "F19":80, "F20":90, "F5":96, "F6":97,
        "F7":98, "F3":99, "F8":100, "F9":101, "F11":103, "F13":105,
        "F16":106, "F14":107, "F10":109, "F12":111, "F15":113,
        "HOME":115, "PAGEUP":116, "END":119, "F2":120,
        "PAGEDOWN":121, "F1":122, "LEFT":123, "RIGHT":124,
        "DOWN":125, "UP":126, "F4":118
    ]
    static func parse(_ text: String) throws -> MouseAction {
        let value = text.trimmingCharacters(in: .whitespacesAndNewlines)
        if value == "默认" { return .original }
        if value == "禁用" { return .disabled }
        if let action = SystemAction(rawValue:value) { return .system(action) }
        let parts = value.uppercased().components(separatedBy: "+").map {
            $0.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        var flags = CGEventFlags()
        for part in parts.dropLast() {
            let flag: CGEventFlags
            switch part {
            case "CMD", "COMMAND", "WIN": flag = .maskCommand
            case "CTRL", "CONTROL": flag = .maskControl
            case "ALT", "OPTION": flag = .maskAlternate
            case "SHIFT": flag = .maskShift
            default: throw InputError.invalid("未知修饰键：\(part)")
            }
            guard !flags.contains(flag) else { throw InputError.invalid("修饰键重复：\(part)") }
            flags.insert(flag)
        }
        guard let last = parts.last, let key = keys[last] else {
            throw InputError.invalid("无效快捷键：\(text)。支持 Cmd、Option、Ctrl、Shift 和 F1～F20。")
        }
        return .shortcut(key, flags)
    }
}

enum InputError: LocalizedError {
    case invalid(String)
    var errorDescription: String? { if case .invalid(let s) = self { return s }; return nil }
}

struct InputPermissions {
    var accessibility: Bool { AXIsProcessTrusted() }
    var listen: Bool { CGPreflightListenEventAccess() }
    var post: Bool { CGPreflightPostEventAccess() }
    var ready: Bool { accessibility && listen && post }
    var dictionary: [String: Any] {
        ["accessibility":accessibility, "inputMonitoring":listen, "postEvents":post]
    }
    func request() {
        let prompt = kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String
        _ = AXIsProcessTrustedWithOptions([prompt: true] as CFDictionary)
        _ = CGRequestListenEventAccess()
        _ = CGRequestPostEventAccess()
    }
}

/// Tracks only swallowed physical button presses, never pointer coordinates or typed keys.
struct ButtonPairs {
    private(set) var swallowed = Set<Int64>()
    mutating func down(button: Int64, shouldSuppress: Bool) -> Bool {
        if shouldSuppress { swallowed.insert(button) }
        return shouldSuppress
    }
    mutating func up(button: Int64) -> Bool { swallowed.remove(button) != nil }
    func drag(button: Int64) -> Bool { swallowed.contains(button) }
}

final class MouseInput {
    private var tap: CFMachPort?
    private var source: CFRunLoopSource?
    private var actions: [Int64: MouseAction] = [:]
    private var pairs = ButtonPairs()
    private var revision = 0
    private(set) var active = false
    private(set) var status = "按键映射未启用"
    var onStateChange: (() -> Void)?
    var onActionError: ((String) -> Void)?
    private let marker: Int64 = 0x4c4f47494c495445

    func configure(_ config: MouseConfig) throws {
        let parsed = try [Int64(3):ShortcutParser.parse(config.back),
                          Int64(4):ShortcutParser.parse(config.forward),
                          Int64(2):ShortcutParser.parse(config.middle)]
        revision += 1
        actions = parsed
        guard config.enabled else { pause("按键映射已暂停"); return }
        guard InputPermissions().ready else {
            pause("权限不足：请授权辅助功能与输入监控；授权后可能需要退出重开")
            return
        }
        if tap == nil { try install() }
        active = true
        status = "映射运行中（所有鼠标）"
        if let tap { CGEvent.tapEnable(tap: tap, enable: true) }
        onStateChange?()
    }

    func pause(_ reason: String = "按键映射已暂停") {
        revision += 1
        active = false
        status = reason
        // Keep the tap until existing swallowed downs receive their matching ups.
        onStateChange?()
    }

    private func install() throws {
        let types: [CGEventType] = [.otherMouseDown,.otherMouseUp,.otherMouseDragged,.keyDown]
        let mask = types.reduce(CGEventMask(0)) { $0 | (1 << $1.rawValue) }
        let pointer = Unmanaged.passUnretained(self).toOpaque()
        guard let installed = CGEvent.tapCreate(tap: .cgSessionEventTap,
            place: .headInsertEventTap, options: .defaultTap, eventsOfInterest: mask,
            callback: { _, type, event, info in
                guard let info else { return Unmanaged.passUnretained(event) }
                return Unmanaged<MouseInput>.fromOpaque(info).takeUnretainedValue().handle(type, event)
            }, userInfo: pointer) else {
            throw InputError.invalid("无法创建输入监听，请检查辅助功能和输入监控权限，然后退出重开。")
        }
        tap = installed
        source = CFMachPortCreateRunLoopSource(kCFAllocatorDefault, installed, 0)
        CFRunLoopAddSource(CFRunLoopGetMain(), source, .commonModes)
        CGEvent.tapEnable(tap: installed, enable: true)
    }

    private func handle(_ type: CGEventType, _ event: CGEvent) -> Unmanaged<CGEvent>? {
        if type == .tapDisabledByTimeout || type == .tapDisabledByUserInput {
            pause("输入监听被系统停用；映射已暂停，请确认后重新启用")
            if let tap { CGEvent.tapEnable(tap: tap, enable: true) }
            return Unmanaged.passUnretained(event)
        }
        if event.getIntegerValueField(.eventSourceUserData) == marker {
            return Unmanaged.passUnretained(event)
        }
        if type == .keyDown {
            let emergency: CGEventFlags = [.maskCommand,.maskControl,.maskShift]
            if event.getIntegerValueField(.keyboardEventKeycode) == 111,
                event.flags.intersection(emergency) == emergency {
                pause("紧急暂停：Cmd+Ctrl+Shift+F12")
            }
            return Unmanaged.passUnretained(event)
        }
        let button = event.getIntegerValueField(.mouseEventButtonNumber)
        switch type {
        case .otherMouseUp:
            return pairs.up(button: button) ? nil : Unmanaged.passUnretained(event)
        case .otherMouseDragged:
            return pairs.drag(button: button) ? nil : Unmanaged.passUnretained(event)
        case .otherMouseDown:
            // A repeated down for a swallowed press must remain swallowed even after pause.
            if pairs.drag(button: button) { return nil }
            guard active, let action = actions[button] else { return Unmanaged.passUnretained(event) }
            if case .original = action { return Unmanaged.passUnretained(event) }
            _ = pairs.down(button: button, shouldSuppress: true)
            perform(action)
            return nil
        default: return Unmanaged.passUnretained(event)
        }
    }

    private func perform(_ action: MouseAction) {
        if case .shortcut(let key, let flags) = action { post(key, flags) }
        if case .system(let systemAction) = action {
            let expectedRevision = revision
            systemAction.open(shouldOpen:{ [weak self] in
                self?.active == true && self?.revision == expectedRevision
            }, onFailure:{ [weak self] error in self?.onActionError?(error) })
        }
    }

    private func post(_ key: CGKeyCode, _ flags: CGEventFlags) {
        guard let down = CGEvent(keyboardEventSource: nil, virtualKey: key, keyDown: true),
              let up = CGEvent(keyboardEventSource: nil, virtualKey: key, keyDown: false) else { return }
        for event in [down,up] {
            event.flags = flags
            event.setIntegerValueField(.eventSourceUserData, value: marker)
            event.post(tap: .cghidEventTap)
        }
    }

    deinit {
        if let source { CFRunLoopRemoveSource(CFRunLoopGetMain(), source, .commonModes) }
        if let tap { CFMachPortInvalidate(tap) }
    }
}

import AppKit
import ApplicationServices

struct MouseConfig: Codable {
    var version = 1
    var enabled = false
    var back = "默认"
    var forward = "默认"
    var middle = "默认"
    var horizontalScroll = HorizontalScrollConfig()

    enum CodingKeys: String, CodingKey { case version, enabled, back, forward, middle, horizontalScroll }
    init() {}
    init(from decoder: Decoder) throws {
        let values = try decoder.container(keyedBy:CodingKeys.self)
        version = try values.decode(Int.self, forKey:.version)
        enabled = try values.decode(Bool.self, forKey:.enabled)
        back = try values.decode(String.self, forKey:.back)
        forward = try values.decode(String.self, forKey:.forward)
        middle = try values.decode(String.self, forKey:.middle)
        horizontalScroll = try values.decodeIfPresent(HorizontalScrollConfig.self, forKey:.horizontalScroll) ?? HorizontalScrollConfig()
        if back.trimmingCharacters(in:.whitespacesAndNewlines) == "水平滚动" { back = "默认"; horizontalScroll.back = true }
        if forward.trimmingCharacters(in:.whitespacesAndNewlines) == "水平滚动" { forward = "默认"; horizontalScroll.forward = true }
        if middle.trimmingCharacters(in:.whitespacesAndNewlines) == "水平滚动" { middle = "默认" }
    }
}

struct HorizontalScrollConfig: Codable {
    var back = false
    var forward = false
    func enabled(for button: Int64) -> Bool {
        switch button { case 3:return back; case 4:return forward; default:return false }
    }
}

/// Read-only application details shared by a newly selected app and saved mappings.
enum ApplicationMetadata {
    static func read(path: String) -> [String: Any] {
        let url = URL(fileURLWithPath:path).standardizedFileURL
        let fallback = url.deletingPathExtension().lastPathComponent
        var result: [String: Any] = ["ok":true, "path":url.path, "name":fallback,
            "value":"打开应用:\(url.path)", "icon":"", "available":false]
        guard path.hasPrefix("/"), url.pathExtension == "app",
              let bundle = Bundle(url:url), bundle.bundleIdentifier != nil,
              let executable = bundle.executableURL,
              FileManager.default.fileExists(atPath:executable.path) else { return result }
        let name = bundle.object(forInfoDictionaryKey:"CFBundleDisplayName") as? String
            ?? bundle.object(forInfoDictionaryKey:"CFBundleName") as? String ?? fallback
        result["name"] = name
        result["available"] = true
        let icon = NSWorkspace.shared.icon(forFile:url.path)
        if let bitmap = NSBitmapImageRep(bitmapDataPlanes:nil, pixelsWide:64, pixelsHigh:64,
            bitsPerSample:8, samplesPerPixel:4, hasAlpha:true, isPlanar:false,
            colorSpaceName:.deviceRGB, bytesPerRow:0, bitsPerPixel:0),
           let context = NSGraphicsContext(bitmapImageRep:bitmap) {
            NSGraphicsContext.saveGraphicsState()
            NSGraphicsContext.current = context
            icon.draw(in:NSRect(x:0,y:0,width:64,height:64), from:.zero, operation:.copy, fraction:1)
            context.flushGraphics()
            NSGraphicsContext.restoreGraphicsState()
            if let png = bitmap.representation(using:.png, properties:[:]) {
                result["icon"] = "data:image/png;base64," + png.base64EncodedString()
            }
        }
        return result
    }
}

enum MouseAction {
    case original, disabled, shortcut(CGKeyCode, CGEventFlags)
    case system(SystemAction)
    case openApplication(URL)
    case media(MediaAction)
}

enum MediaAction: String, CaseIterable {
    case volumeUp = "音量增加", volumeDown = "音量减小", mute = "静音切换"
    case playPause = "播放或暂停", nextTrack = "下一首", previousTrack = "上一首"
    case brightnessUp = "亮度增加", brightnessDown = "亮度减小"

    var key: Int {
        switch self {
        case .volumeUp:return 0; case .volumeDown:return 1
        case .brightnessUp:return 2; case .brightnessDown:return 3
        case .mute:return 7; case .playPause:return 16
        case .nextTrack:return 17; case .previousTrack:return 18
        }
    }
    func event(down: Bool, marker: Int64) -> CGEvent? {
        let state = down ? 0xa : 0xb
        let event = NSEvent.otherEvent(with:.systemDefined, location:.zero,
            modifierFlags:NSEvent.ModifierFlags(rawValue:UInt(state << 8)),
            timestamp:0, windowNumber:0, context:nil, subtype:8,
            data1:(key << 16) | (state << 8), data2:-1)?.cgEvent
        event?.setIntegerValueField(.eventSourceUserData, value:marker)
        return event
    }
}

enum SystemAction: String, CaseIterable {
    case apps = "打开 App"
    case missionControl = "调度中心"
    case settings = "系统设置"
    case calculator = "计算器"

    var bundleIdentifiers: [String] {
        switch self {
        case .apps: return ["com.apple.apps.launcher", "com.apple.launchpad.launcher"]
        case .missionControl: return ["com.apple.exposelauncher"]
        case .settings: return ["com.apple.systempreferences"]
        case .calculator: return ["com.apple.calculator"]
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

enum ActionCatalog {
    static let categories: [[String: Any]] = [
        ["id":"system", "label":"系统功能", "actions":[
            ["id":"default", "label":"保留默认行为", "value":"默认"],
            ["id":"disabled", "label":"禁用按键", "value":"禁用"],
            ["id":"apps", "label":"打开 App", "value":"打开 App"],
            ["id":"mission-control", "label":"调度中心", "value":"调度中心"],
            ["id":"settings", "label":"系统设置", "value":"系统设置"],
            ["id":"calculator", "label":"计算器", "value":"计算器"],
            ["id":"lock", "label":"锁定屏幕", "value":"Ctrl+Cmd+Q"]
        ]],
        ["id":"editing", "label":"编辑", "actions":[
            ["id":"copy", "label":"复制", "value":"Cmd+C"], ["id":"paste", "label":"粘贴", "value":"Cmd+V"],
            ["id":"cut", "label":"剪切", "value":"Cmd+X"], ["id":"undo", "label":"撤销", "value":"Cmd+Z"],
            ["id":"redo", "label":"重做", "value":"Cmd+Shift+Z"], ["id":"select-all", "label":"全选", "value":"Cmd+A"],
            ["id":"save", "label":"保存", "value":"Cmd+S"], ["id":"print", "label":"打印", "value":"Cmd+P" ]
        ]],
        ["id":"windows", "label":"窗口与桌面", "actions":[
            ["id":"close", "label":"关闭窗口或标签页", "value":"Cmd+W"], ["id":"minimize", "label":"最小化窗口", "value":"Cmd+M"],
            ["id":"switch-app", "label":"切换应用", "value":"Cmd+Tab"], ["id":"mission-key", "label":"调度中心快捷键", "value":"Ctrl+Up"],
            ["id":"app-windows", "label":"当前应用的窗口", "value":"Ctrl+Down"], ["id":"desktop-left", "label":"左侧桌面", "value":"Ctrl+Left"],
            ["id":"desktop-right", "label":"右侧桌面", "value":"Ctrl+Right"], ["id":"show-desktop", "label":"显示桌面", "value":"F11"]
        ]],
        ["id":"browser", "label":"浏览器", "actions":[
            ["id":"new-tab", "label":"新建标签页", "value":"Cmd+T"], ["id":"close-tab", "label":"关闭标签页", "value":"Cmd+W"],
            ["id":"restore-tab", "label":"恢复关闭的标签页", "value":"Cmd+Shift+T"], ["id":"next-tab", "label":"下一个标签页", "value":"Ctrl+Tab"],
            ["id":"previous-tab", "label":"上一个标签页", "value":"Ctrl+Shift+Tab"], ["id":"refresh", "label":"刷新页面", "value":"Cmd+R"]
        ]],
        ["id":"media", "label":"媒体与截图", "actions":[
            ["id":"volume-up", "label":"音量增加", "value":"音量增加"], ["id":"volume-down", "label":"音量减小", "value":"音量减小"],
            ["id":"mute", "label":"静音切换", "value":"静音切换"], ["id":"play-pause", "label":"播放或暂停", "value":"播放或暂停"],
            ["id":"next-track", "label":"下一首", "value":"下一首"], ["id":"previous-track", "label":"上一首", "value":"上一首"],
            ["id":"brightness-up", "label":"亮度增加", "value":"亮度增加"], ["id":"brightness-down", "label":"亮度减小", "value":"亮度减小"],
            ["id":"fullscreen-shot", "label":"截取全屏", "value":"Cmd+Shift+3"], ["id":"area-shot", "label":"区域截图", "value":"Cmd+Shift+4"],
            ["id":"screenshot-toolbar", "label":"截屏与录屏工具栏", "value":"Cmd+Shift+5"], ["id":"emoji", "label":"表情与符号", "value":"Ctrl+Cmd+Space"],
            ["id":"spotlight", "label":"聚焦搜索", "value":"Cmd+Space"]
        ]]
    ]
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
        if let action = MediaAction(rawValue:value) { return .media(action) }
        if value.hasPrefix("打开应用:") {
            let path = String(value.dropFirst("打开应用:".count))
            guard path.hasPrefix("/"), path.hasSuffix(".app") else {
                throw InputError.invalid("应用必须是绝对路径的 .app")
            }
            let url = URL(fileURLWithPath:path).standardizedFileURL
            return .openApplication(url)
        }
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

/// A side click is deferred until release; wheel motion consumes it instead.
/// This state contains no input events and can be verified without posting input.
struct HeldSideButtons {
    private var consumed: [Int64: Bool] = [:]
    mutating func begin(button: Int64) {
        guard button == 3 || button == 4 else { return }
        if consumed[button] == nil { consumed[button] = false }
    }
    mutating func scroll(hasVerticalMotion: Bool) -> Bool {
        guard hasVerticalMotion, !consumed.isEmpty else { return false }
        for button in Array(consumed.keys) { consumed[button] = true }
        return true
    }
    /// nil means this was not a deferred press; true means execute its saved click.
    mutating func release(button: Int64) -> Bool? {
        consumed.removeValue(forKey:button).map { !$0 }
    }
    mutating func cancel() { consumed.removeAll() }
}

/// Change only the axes so pixel/line deltas, phases and momentum remain intact.
enum HorizontalWheel {
    static func hasVerticalMotion(_ event: CGEvent) -> Bool {
        [.scrollWheelEventDeltaAxis1, .scrollWheelEventPointDeltaAxis1, .scrollWheelEventFixedPtDeltaAxis1]
            .contains { event.getDoubleValueField($0) != 0 }
    }
    static func convert(_ event: CGEvent) {
        let axes: [(CGEventField, CGEventField)] = [
            (.scrollWheelEventDeltaAxis1, .scrollWheelEventDeltaAxis2),
            (.scrollWheelEventPointDeltaAxis1, .scrollWheelEventPointDeltaAxis2),
            (.scrollWheelEventFixedPtDeltaAxis1, .scrollWheelEventFixedPtDeltaAxis2)
        ]
        // Point delta setters also update fixed-point deltas. Snapshot first and
        // write fixed-point last to retain sub-pixel wheel motion.
        let values = axes.map { event.getDoubleValueField($0.1) + event.getDoubleValueField($0.0) }
        for ((vertical, horizontal), delta) in zip(axes, values) {
            event.setDoubleValueField(horizontal, value:delta)
            event.setDoubleValueField(vertical, value:0)
        }
    }
}

final class MouseInput {
    private var tap: CFMachPort?
    private var source: CFRunLoopSource?
    private var actions: [Int64: MouseAction] = [:]
    private var pairs = ButtonPairs()
    private var horizontalSettings = HorizontalScrollConfig()
    private var heldSideButtons = HeldSideButtons()
    private var pendingOriginalClicks: [Int64: CGEvent] = [:]
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
        heldSideButtons.cancel()
        pendingOriginalClicks.removeAll()
        actions = parsed
        horizontalSettings = config.horizontalScroll
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
        heldSideButtons.cancel()
        pendingOriginalClicks.removeAll()
        active = false
        status = reason
        // Keep the tap until existing swallowed downs receive their matching ups.
        onStateChange?()
    }

    private func install() throws {
        let types: [CGEventType] = [.otherMouseDown,.otherMouseUp,.otherMouseDragged,.scrollWheel,.keyDown]
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
        case .scrollWheel:
            guard active, heldSideButtons.scroll(hasVerticalMotion:HorizontalWheel.hasVerticalMotion(event)) else {
                return Unmanaged.passUnretained(event)
            }
            HorizontalWheel.convert(event)
            return Unmanaged.passUnretained(event)
        case .otherMouseUp:
            let shouldClick = heldSideButtons.release(button:button)
            let originalDown = pendingOriginalClicks.removeValue(forKey:button)
            let suppress = pairs.up(button:button)
            if active, shouldClick == true, let action = actions[button] {
                if case .original = action {
                    if let down = originalDown, let up = event.copy() {
                        for click in [down, up] {
                            click.setIntegerValueField(.eventSourceUserData, value:marker)
                            click.post(tap:.cghidEventTap)
                        }
                    }
                } else { perform(action) }
            }
            return suppress ? nil : Unmanaged.passUnretained(event)
        case .otherMouseDragged:
            return pairs.drag(button: button) ? nil : Unmanaged.passUnretained(event)
        case .otherMouseDown:
            // A repeated down for a swallowed press must remain swallowed even after pause.
            if pairs.drag(button: button) { return nil }
            guard active, let action = actions[button] else { return Unmanaged.passUnretained(event) }
            if horizontalSettings.enabled(for:button) {
                if case .original = action {
                    guard let copy = event.copy() else { return Unmanaged.passUnretained(event) }
                    pendingOriginalClicks[button] = copy
                }
                _ = pairs.down(button:button, shouldSuppress:true)
                heldSideButtons.begin(button:button)
                return nil
            }
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
        if case .openApplication(let url) = action {
            let expectedRevision = revision
            DispatchQueue.main.async { [weak self] in
                guard let self, self.active, self.revision == expectedRevision else { return }
                var isDirectory = ObjCBool(false)
                guard FileManager.default.fileExists(atPath:url.path, isDirectory:&isDirectory), isDirectory.boolValue,
                      Bundle(url:url) != nil else { self.onActionError?("找不到有效的应用：\(url.path)"); return }
                let options = NSWorkspace.OpenConfiguration(); options.activates = true
                NSWorkspace.shared.openApplication(at:url, configuration:options) { _, error in
                    if let error { DispatchQueue.main.async { self.onActionError?("无法打开应用：\(error.localizedDescription)") } }
                }
            }
        }
        if case .media(let media) = action {
            guard let down = media.event(down:true, marker:marker), let up = media.event(down:false, marker:marker) else {
                onActionError?("无法发送媒体动作：\(media.rawValue)")
                return
            }
            down.post(tap:.cghidEventTap)
            up.post(tap:.cghidEventTap)
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

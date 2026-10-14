import AppKit
import ApplicationServices
import Darwin

@_silgen_name("logilite_load_config")
private func loadConfigJSON() -> UnsafeMutablePointer<CChar>?
@_silgen_name("logilite_save_config")
private func saveConfigJSON(_ json: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?
@_silgen_name("logilite_free_string")
private func freeConfigJSON(_ pointer: UnsafeMutablePointer<CChar>)

private func encoded(_ value: Any) -> String {
    guard JSONSerialization.isValidJSONObject(value),
          let data = try? JSONSerialization.data(withJSONObject:value, options:[.sortedKeys]),
          let text = String(data:data, encoding:.utf8) else { return "{\"ok\":false,\"error\":\"JSON encoding failed\"}" }
    return text
}

private enum DesktopConfigStore {
    static func consume(_ pointer: UnsafeMutablePointer<CChar>?) throws -> MouseConfig {
        guard let pointer else { throw InputError.invalid("配置接口无返回") }
        defer { freeConfigJSON(pointer) }
        let data = Data(String(cString:pointer).utf8)
        let result = try JSONSerialization.jsonObject(with:data) as? [String:Any] ?? [:]
        guard result["ok"] as? Bool == true, let config = result["config"] else {
            throw InputError.invalid(result["error"] as? String ?? "配置读取失败")
        }
        return try JSONDecoder().decode(MouseConfig.self, from:JSONSerialization.data(withJSONObject:config))
    }
    static func load() throws -> MouseConfig { try consume(loadConfigJSON()) }
    static func save(_ config: MouseConfig) throws -> MouseConfig {
        let text = String(decoding:try JSONEncoder().encode(config), as:UTF8.self)
        return try text.withCString { try consume(saveConfigJSON($0)) }
    }
}

/// Only called on the app's main thread. HID operations use a separate Rust worker.
private final class DesktopRuntime {
    static let shared = DesktopRuntime()
    let input = MouseInput()
    var config = MouseConfig()
    var error: String?
    var smoke = false
    var started = false
    var needsOnboarding = false
    var appIconLoaded = false
    var activation = MappingActivationPolicy(defaultPending:false)
    var configLoaded = false
    var shuttingDown = false
    private let mappingChoiceKey = "desktop.mappingChoice.v1"

    func start(smoke: Bool) {
        guard !started else { return }
        started = true; self.smoke = smoke
        // Apply the bundled artwork to the running app instead of relying on
        // Launch Services' cached icon for this repeatedly rebuilt bundle.
        if let url = Bundle.main.url(forResource:"app-logo", withExtension:"png"),
           let icon = NSImage(contentsOf:url) {
            NSApp.applicationIconImage = icon
            appIconLoaded = true
        }
        if smoke { return }
        do { config = try DesktopConfigStore.load(); configLoaded = true } catch { self.error = error.localizedDescription }
        activation = MappingActivationPolicy(defaultPending:!UserDefaults.standard.bool(forKey:mappingChoiceKey))
        needsOnboarding = !UserDefaults.standard.bool(forKey:"desktop.permissionOnboarding.v1") && !InputPermissions().ready
        input.onStateChange = { [weak self] in
            guard let self, !self.input.active, !self.shuttingDown, InputPermissions().ready else { return }
            DispatchQueue.main.async { [weak self] in self?.persistPause() }
        }
        input.onActionError = { [weak self] in self?.error = $0 }
        if !config.enabled { do { try input.configure(config) } catch { self.error = error.localizedDescription } }
        reconcilePermissions(InputPermissions().ready)
    }
    func resolveMappingChoice() {
        activation.resolveDefault()
        UserDefaults.standard.set(true, forKey:mappingChoiceKey)
    }
    func reconcilePermissions(_ ready: Bool) {
        guard !smoke, !shuttingDown, configLoaded else { return }
        if !ready && input.active { input.pause("权限已撤销，等待重新授权") }
        guard activation.shouldEnable(ready:ready, enabled:config.enabled, active:input.active) else { return }
        do {
            var updated = config; updated.enabled = true
            try input.configure(updated)
            guard input.active else { return }
            config = try DesktopConfigStore.save(updated)
            resolveMappingChoice()
            error = nil
        } catch {
            input.pause()
            self.error = error.localizedDescription
        }
    }
    func persistPause() {
        guard !smoke, !shuttingDown, InputPermissions().ready, !input.active, config.enabled else { return }
        resolveMappingChoice()
        config.enabled = false
        do { config = try DesktopConfigStore.save(config) } catch { self.error = error.localizedDescription }
    }
    func snapshot() -> [String:Any] {
        let permissions = InputPermissions()
        reconcilePermissions(permissions.ready)
        let data = try? JSONEncoder().encode(config)
        let configValue = data.flatMap { try? JSONSerialization.jsonObject(with:$0) } ?? [:]
        return ["ok":true, "config":configValue, "active":input.active,
                "inputStatus":input.status, "permissions":permissions.dictionary,
                "onboarding":needsOnboarding, "smoke":smoke, "appIconLoaded":appIconLoaded, "error":error ?? ""]
    }
    func perform(_ request: [String:Any]) throws -> [String:Any] {
        let action = request["action"] as? String ?? "state"
        if action == "init" { start(smoke:request["smoke"] as? Bool ?? false) }
        if smoke && !["state", "init", "metadata"].contains(action) { throw InputError.invalid("自检模式不会修改配置或权限") }
        switch action {
        case "init", "state": break
        case "save":
            guard let value = request["config"] else { throw InputError.invalid("缺少配置") }
            var updated = try JSONDecoder().decode(MouseConfig.self, from:JSONSerialization.data(withJSONObject:value))
            _ = try ShortcutParser.parse(updated.back); _ = try ShortcutParser.parse(updated.forward); _ = try ShortcutParser.parse(updated.middle)
            if updated.enabled && !InputPermissions().ready { throw InputError.invalid("请先在设置中完成辅助功能和输入监控授权") }
            updated = try DesktopConfigStore.save(updated)
            config = updated
            do { try input.configure(updated) } catch { input.pause(); persistPause(); throw error }
            error = nil
        case "shutdown": shuttingDown = true; input.pause()
        case "pause": resolveMappingChoice(); input.pause(); persistPause()
        case "toggle":
            if input.active { resolveMappingChoice(); input.pause(); persistPause() }
            else {
                guard InputPermissions().ready else { throw InputError.invalid("请先完成输入权限授权") }
                var updated = config; updated.enabled = true
                config = try DesktopConfigStore.save(updated)
                do { try input.configure(config) } catch { input.pause(); persistPause(); throw error }
                resolveMappingChoice()
            }
        case "requestPermissions":
            UserDefaults.standard.set(true, forKey:"desktop.permissionOnboarding.v1")
            needsOnboarding = false
            InputPermissions().request()
        case "dismissOnboarding":
            UserDefaults.standard.set(true, forKey:"desktop.permissionOnboarding.v1")
            needsOnboarding = false
        case "settings":
            let pane = request["pane"] as? String == "input" ? "Privacy_ListenEvent" : "Privacy_Accessibility"
            if let url = URL(string:"x-apple.systempreferences:com.apple.preference.security?\(pane)") { NSWorkspace.shared.open(url) }
        case "metadata": return ["ok":true, "devices":HardwareService().discover()]
        case "export":
            let panel = NSSavePanel(); panel.nameFieldStringValue = "LogiLite-diagnostics.json"
            if panel.runModal() == .OK, let url = panel.url {
                let output: [String:Any] = ["app":"LogiLite", "runtime":snapshot(), "devices":HardwareService().discover(),
                    "hardware":request["hardware"] ?? [:], "note":"No pointer paths, typed keys or input history are recorded."]
                try encoded(output).write(to:url, atomically:true, encoding:.utf8)
            }
        default: throw InputError.invalid("未知操作")
        }
        return snapshot()
    }
}

@_cdecl("ll_desktop_request")
func desktopRequest(_ json: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>? {
    precondition(Thread.isMainThread)
    do {
        let request = try JSONSerialization.jsonObject(with:Data(String(cString:json).utf8)) as? [String:Any] ?? [:]
        return strdup(encoded(try DesktopRuntime.shared.perform(request)))
    } catch { return strdup(encoded(["ok":false,"error":error.localizedDescription])) }
}

@_cdecl("ll_desktop_hardware")
func desktopHardware(_ json: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>? {
    do {
        let request = try JSONSerialization.jsonObject(with:Data(String(cString:json).utf8)) as? [String:Any] ?? [:]
        return strdup(encoded(HardwareService().perform(command:request)))
    } catch { return strdup(encoded(["ok":false,"error":error.localizedDescription])) }
}

@_cdecl("ll_desktop_free")
func desktopFree(_ pointer: UnsafeMutablePointer<CChar>?) { free(pointer) }

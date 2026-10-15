import Foundation
import IOKit
import IOKit.hid

private typealias ExchangeCallback = @convention(c) (UnsafeMutableRawPointer?, UnsafePointer<UInt8>?, Int, UnsafeMutablePointer<UInt8>?, Int) -> Int32
@_silgen_name("logilite_hardware")
private func rustHardware(_ context: UnsafeMutableRawPointer?, _ callback: ExchangeCallback?, _ json: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>?
@_silgen_name("logilite_free_string")
private func freeHardwareJSON(_ pointer: UnsafeMutablePointer<CChar>)

// Parse the actual report descriptor, including PUSH/POP; primary mouse usage
// does not exclude a vendor collection on Logitech Bluetooth devices.
struct HIDReportLayout {
    var inputs: [Int: Int] = [:]
    var outputs: [Int: Int] = [:]
    init(_ data: Data) {
        let bytes = [UInt8](data)
        var offset = 0, size = 0, count = 0, report = 0
        var stack: [(Int, Int, Int)] = []
        var inBits: [Int: Int] = [:], outBits: [Int: Int] = [:]
        while offset < bytes.count {
            let prefix = bytes[offset]; offset += 1
            if prefix == 0xfe {
                guard offset + 2 <= bytes.count else { return }
                let length = Int(bytes[offset]); offset += 2
                guard offset + length <= bytes.count else { return }
                offset += length; continue
            }
            let length = [0, 1, 2, 4][Int(prefix & 3)]
            guard offset + length <= bytes.count else { return }
            var value = 0
            for i in 0..<length { value |= Int(bytes[offset + i]) << (8 * i) }
            offset += length
            let type = (prefix >> 2) & 3, tag = prefix >> 4
            if type == 1 {
                switch tag {
                case 7: size = value
                case 8: report = value
                case 9: count = value
                case 10: stack.append((size, count, report))
                case 11:
                    guard let saved = stack.popLast() else { return }
                    (size, count, report) = saved
                default: break
                }
            } else if type == 0 && size <= 64 && count <= 4096 {
                if tag == 8 { inBits[report, default: 0] += size * count }
                if tag == 9 { outBits[report, default: 0] += size * count }
            }
        }
        inputs = inBits.mapValues { ($0 + 7) / 8 }
        outputs = outBits.mapValues { ($0 + 7) / 8 }
        for id in inputs.keys where id != 0 { inputs[id]! += 1 }
        for id in outputs.keys where id != 0 { outputs[id]! += 1 }
    }
    var supportsLongReport: Bool { (inputs[17] ?? 0) == 20 && (outputs[17] ?? 0) == 20 }
}

private final class HIDEndpoint {
    let device: IOHIDDevice
    let layout: HIDReportLayout
    let product: String
    let pid: Int
    let registryID: UInt64
    init(_ device: IOHIDDevice) {
        self.device = device
        product = IOHIDDeviceGetProperty(device, kIOHIDProductKey as CFString) as? String ?? "Logitech"
        pid = (IOHIDDeviceGetProperty(device, kIOHIDProductIDKey as CFString) as? NSNumber)?.intValue ?? 0
        layout = HIDReportLayout(IOHIDDeviceGetProperty(device, kIOHIDReportDescriptorKey as CFString) as? Data ?? Data())
        var id: UInt64 = 0
        IORegistryEntryGetRegistryEntryID(IOHIDDeviceGetService(device), &id)
        registryID = id
    }
    var isDirectTarget: Bool {
        let name = product.lowercased().filter { !" -_".contains($0) }
        return name.hasPrefix("mxanywhere3") && layout.supportsLongReport
    }
    // Keep reconnect identity in memory only; never include it in diagnostics.
    var reconnectIdentity: String? {
        for key in [kIOHIDUniqueIDKey, kIOHIDSerialNumberKey] {
            if let value = IOHIDDeviceGetProperty(device, key as CFString) {
                let text = String(describing:value)
                if !text.isEmpty && text != "0" { return "\(pid):\(text)" }
            }
        }
        return nil
    }
    var metadata: [String: Any] {
        ["product": product, "vid": 0x046d, "pid": pid,
         "registry_id": String(registryID),
         "transport": IOHIDDeviceGetProperty(device, kIOHIDTransportKey as CFString) as? String ?? "unknown",
         "usage_page": (IOHIDDeviceGetProperty(device, kIOHIDPrimaryUsagePageKey as CFString) as? NSNumber)?.intValue ?? 0,
         "usage": (IOHIDDeviceGetProperty(device, kIOHIDPrimaryUsageKey as CFString) as? NSNumber)?.intValue ?? 0,
         "input_report_lengths": Dictionary(uniqueKeysWithValues: layout.inputs.map { (String($0.key), $0.value) }),
         "output_report_lengths": Dictionary(uniqueKeysWithValues: layout.outputs.map { (String($0.key), $0.value) }),
         "hidpp_candidate": isDirectTarget]
    }
}

private final class HIDWrite {
    let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: 20)
    var result: IOReturn?
    init(_ request: [UInt8]) { buffer.initialize(from: request, count: 20) }
    deinit { buffer.deinitialize(count: 20); buffer.deallocate() }
}

private final class HIDSession {
    let endpoint: HIDEndpoint
    let runLoop: CFRunLoop
    let buffer = UnsafeMutablePointer<UInt8>.allocate(capacity: 64)
    var opened = false
    var pending: [UInt8] = []
    var response: [UInt8]?
    var lastError: String?
    var deadline: Date
    var onNotification: (([UInt8]) -> Void)?
    init(_ endpoint: HIDEndpoint, deadline: Date) {
        self.endpoint = endpoint; self.deadline = deadline
        runLoop = CFRunLoopGetCurrent()
        buffer.initialize(repeating: 0, count: 64)
    }
    deinit {
        if opened {
            IOHIDDeviceUnscheduleFromRunLoop(endpoint.device, runLoop, CFRunLoopMode.defaultMode.rawValue)
            IOHIDDeviceRegisterInputReportCallback(endpoint.device, buffer, 64, nil, nil)
            IOHIDDeviceClose(endpoint.device, IOOptionBits(0))
        }
        buffer.deinitialize(count: 64); buffer.deallocate()
    }
    func open() -> Bool {
        let result = IOHIDDeviceOpen(endpoint.device, IOOptionBits(0))
        guard result == kIOReturnSuccess else {
            lastError = "无法打开鼠标 HID 端点（\(result)），请检查输入监控权限或 Options+ 占用。"; return false
        }
        opened = true
        IOHIDDeviceRegisterInputReportCallback(endpoint.device, buffer, 64, { context, result, _, _, reportID, report, length in
            guard let context, result == kIOReturnSuccess, length > 0, length <= 64 else { return }
            let session = Unmanaged<HIDSession>.fromOpaque(context).takeUnretainedValue()
            var bytes = Array(UnsafeBufferPointer(start: report, count: length))
            // On macOS Bluetooth, implementations may deliver the ID separately.
            if bytes.count == 19 && reportID == 0x11 { bytes.insert(0x11, at: 0) }
            if bytes.count == 6 && reportID == 0x10 { bytes.insert(0x10, at: 0) }
            if HIDSession.matches(session.pending, bytes) { session.response = bytes }
            else { session.onNotification?(bytes) }
        }, Unmanaged.passUnretained(self).toOpaque())
        IOHIDDeviceScheduleWithRunLoop(endpoint.device, runLoop, CFRunLoopMode.defaultMode.rawValue)
        return true
    }
    static func matches(_ request: [UInt8], _ reply: [UInt8]) -> Bool {
        guard request.count >= 4, reply.count >= 4, [0x10, 0x11].contains(reply[0]),
              reply[1] == request[1] || (request[1] == 255 && reply[1] == 0) else { return false }
        return (reply[2] == request[2] && reply[3] == request[3]) ||
            (reply.count >= 6 && [0x8f, 0xff].contains(reply[2]) && reply[3] == request[2] && reply[4] == request[3])
    }
    func exchange(_ request: [UInt8]) -> [UInt8]? {
        guard Date() < deadline else { lastError = "设备探测达到 12 秒上限，请重新刷新。"; return nil }
        guard request.count == 20, request.first == 0x11 else { lastError = "无效 HID++ 长报告"; return nil }
        // Drain queued notifications/replies before arming a new transaction.
        pending = []; response = nil
        CFRunLoopRunInMode(.defaultMode, 0.002, false)
        pending = request
        let write = HIDWrite(request)
        // Completion owns the buffer independently of the session. If a device
        // vanishes before delivering completion, never free memory still owned
        // by IOKit; a late callback can safely release this small allocation.
        let retainedWrite = Unmanaged.passRetained(write)
        let result = IOHIDDeviceSetReportWithCallback(endpoint.device, kIOHIDReportTypeOutput,
            CFIndex(request[0]), write.buffer, 20, 800, { context, result, _, _, _, _, _ in
                guard let context else { return }
                let write = Unmanaged<HIDWrite>.fromOpaque(context).takeRetainedValue()
                write.result = result
            }, retainedWrite.toOpaque())
        guard result == kIOReturnSuccess else {
            retainedWrite.release()
            lastError = "发送 HID++ 报告失败（\(result)）。"; pending = []; return nil
        }
        let until = min(deadline, Date().addingTimeInterval(0.8))
        while (response == nil || write.result == nil) && Date() < until {
            if let result = write.result, result != kIOReturnSuccess { break }
            CFRunLoopRunInMode(.defaultMode, min(0.015, until.timeIntervalSinceNow), true)
        }
        pending = []
        guard write.result == kIOReturnSuccess else {
            lastError = "发送 HID++ 报告超时或失败（\(write.result.map(String.init) ?? "timeout")）。"; return nil
        }
        guard let response else { lastError = "蓝牙 HID++ 响应超时；移动鼠标唤醒后重试，并检查 Options+ 后台竞争。"; return nil }
        return response
    }
}

final class HardwareService {
    // All methods run serially on a dedicated queue, never on the input tap.
    fileprivate static func endpoints() -> [HIDEndpoint] {
        let manager = IOHIDManagerCreate(kCFAllocatorDefault, IOOptionBits(0))
        IOHIDManagerSetDeviceMatching(manager, [kIOHIDVendorIDKey: 0x046d] as CFDictionary)
        // Matching enumerates metadata without requesting TCC or seizing input.
        let devices = IOHIDManagerCopyDevices(manager) as? Set<IOHIDDevice> ?? []
        return devices.map(HIDEndpoint.init).sorted { $0.registryID < $1.registryID }
    }
    func discover() -> [[String: Any]] { Self.endpoints().map { $0.metadata } }
    func perform(command: [String: Any]) -> [String: Any] {
        HIDWorker.shared.perform(command)
    }
    fileprivate func performOnWorker(command: [String: Any], session: HIDSession? = nil) -> [String: Any] {
        let endpoints = Self.endpoints()
        let devices = endpoints.map { $0.metadata }
        let candidates = endpoints.filter { $0.isDirectTarget }
        guard candidates.count == 1, let endpoint = candidates.first else {
            return ["ok": false, "devices": devices, "error": candidates.count > 1
                ? "发现多只目标鼠标，请只连接要测试的一只后刷新。"
                : "未找到提供 HID++ 长报告的 MX Anywhere 3 蓝牙端点。可先使用按键映射，唤醒鼠标后重试。"]
        }
        let session = session ?? HIDSession(endpoint, deadline: Date().addingTimeInterval(12))
        session.deadline = Date().addingTimeInterval(12)
        guard session.opened || session.open() else { return ["ok": false, "devices": devices, "error": session.lastError ?? "无法打开设备"] }
        var input = command
        input["device_index"] = 255
        guard let data = try? JSONSerialization.data(withJSONObject: input),
              let json = String(data: data, encoding: .utf8) else { return ["ok": false, "error": "无效命令 JSON"] }
        let callback: ExchangeCallback = { context, request, count, output, capacity in
            guard let context, let request, let output, count > 0, count <= 20 else { return -1 }
            let session = Unmanaged<HIDSession>.fromOpaque(context).takeUnretainedValue()
            guard let reply = session.exchange(Array(UnsafeBufferPointer(start: request, count: count))), reply.count <= capacity else { return -2 }
            output.update(from: reply, count: reply.count)
            return Int32(reply.count)
        }
        return withExtendedLifetime(session) {
            guard let pointer = json.withCString({ rustHardware(Unmanaged.passUnretained(session).toOpaque(), callback, $0) }) else {
                return ["ok": false, "error": "Rust 设备桥接无返回"]
            }
            defer { freeHardwareJSON(pointer) }
            var result = (try? JSONSerialization.jsonObject(with: Data(String(cString: pointer).utf8))) as? [String: Any] ?? ["ok": false, "error": "设备返回 JSON 无效"]
            result["devices"] = devices
            if input["action"] as? String == "probe", result["ok"] as? Bool == true {
                let top = TopButtonProtocol(exchange: { [weak session] in session?.exchange($0) })
                do {
                    let feature = try top.discover()
                    let flags = try top.reporting(feature)
                    result["top_button"] = ["supported":true, "control_id":Int(TopButtonProtocol.control), "feature_index":Int(feature), "reporting_flags":Int(flags)]
                } catch {
                    result["top_button"] = ["supported":false, "error":error.localizedDescription]
                }
            }
            if let error = session.lastError { result["transport_error"] = error }
            return result
        }
    }
}

/// HID++ 2.0 REPROG_CONTROLS_V4 (0x1b04). Only temporary diversion of the
/// device-advertised ModeShift control is used; no persistent/raw-motion flags.
final class TopButtonProtocol {
    static let control: UInt16 = 0x00c4
    var softwareID: UInt8 = 7
    let exchange: ([UInt8]) -> [UInt8]?
    init(exchange: @escaping ([UInt8]) -> [UInt8]?) { self.exchange = exchange }

    func request(_ feature: UInt8, _ function: UInt8, _ data: [UInt8] = [], needed: Int) throws -> [UInt8] {
        softwareID = softwareID % 15 + 1
        var packet = [UInt8](repeating:0, count:20)
        packet[0...3] = [0x11, 255, feature, function << 4 | softwareID][0...3]
        packet.replaceSubrange(4..<4 + data.count, with:data)
        guard let reply = exchange(packet), HIDSession.matches(packet, reply),
              reply.count >= (reply.first == 0x11 ? 20 : 7),
              reply[2] == feature, reply[3] == packet[3], reply.count >= 4 + needed else {
            throw HIDTopError.message("顶部按钮 HID++ 响应无效或超时，请唤醒鼠标后重试")
        }
        return Array(reply.dropFirst(4))
    }
    func discover() throws -> UInt8 {
        let root = try request(0, 0, [0x1b, 0x04], needed:3)
        guard root[0] != 0 else { throw HIDTopError.message("此鼠标未提供顶部按钮自定义接口") }
        let feature = root[0]
        let count = try request(feature, 0, needed:1)[0]
        guard count > 0, count <= 64 else { throw HIDTopError.message("鼠标按钮能力数据无效") }
        for index in 0..<count {
            let info = try request(feature, 1, [index], needed:5)
            if UInt16(info[0]) << 8 | UInt16(info[1]) == Self.control {
                guard info[4] & 0x20 != 0 else { throw HIDTopError.message("此鼠标的顶部按钮不支持临时接管") }
                return feature
            }
        }
        throw HIDTopError.message("未找到可自定义的顶部模式按钮")
    }
    func reporting(_ feature: UInt8) throws -> UInt8 {
        let data = try request(feature, 2, [0, 0xc4], needed:5)
        guard data[0] == 0, data[1] == 0xc4 else { throw HIDTopError.message("顶部按钮状态属于其他按键") }
        let mapped = UInt16(data[3]) << 8 | UInt16(data[4])
        guard mapped == 0 || mapped == Self.control else {
            throw HIDTopError.message("顶部按钮已被其他软件改写，请先在 Options+ 恢复默认")
        }
        return data[2]
    }
    func divert(_ feature: UInt8, enabled: Bool) throws {
        let response = try request(feature, 3, [0, 0xc4, enabled ? 3 : 2, 0, 0], needed:2)
        guard response[0] == 0, response[1] == 0xc4,
              (try reporting(feature) & 1 != 0) == enabled else {
            throw HIDTopError.message("鼠标未确认顶部按钮设置，请检查 Options+ 后台占用")
        }
    }
}

private enum HIDTopError: LocalizedError {
    case message(String)
    var errorDescription: String? { if case .message(let text) = self { return text }; return nil }
}

/// A notification is the whole currently pressed CID set, not a click packet.
/// Ignore replies, other slots/features, raw-motion events and malformed reports.
struct TopButtonPressTracker {
    var pressed = false
    mutating func consume(_ bytes: [UInt8], feature: UInt8) -> Bool {
        guard bytes.count == 20, bytes[0] == 0x11, [0, 255].contains(bytes[1]),
              bytes[2] == feature, bytes[3] == 0 else { return false }
        let down = stride(from:4, to:12, by:2).contains {
            UInt16(bytes[$0]) << 8 | UInt16(bytes[$0 + 1]) == TopButtonProtocol.control
        }
        let fired = down && !pressed
        pressed = down
        return fired
    }
}

/// A single run loop owns every open HID++ endpoint and every transaction.
/// The listener stays alive while the window is hidden and reconnects by registry ID.
private final class TopDeliveryGate {
    private let lock = NSLock()
    private var valid = true
    func invalidate() { lock.lock(); valid = false; lock.unlock() }
    var isValid: Bool { lock.lock(); defer { lock.unlock() }; return valid }
}

private final class HIDWorker {
    static let shared = HIDWorker()
    private let ready = NSCondition()
    private var loop: CFRunLoop?
    private var thread: Thread?
    private var session: HIDSession?
    private var feature: UInt8?
    private var tracker = TopButtonPressTracker()
    private var protocolClient: TopButtonProtocol?
    private var wanted = false
    private var diverted = false
    private var delivery = TopDeliveryGate()
    private var ownedIdentities: Set<String> = []
    private var onPress: (() -> Void)?
    private var onStatus: ((String?) -> Void)?
    private var timer: Timer?
    // Prevent timer reentry while exchange pumps the run loop.
    private var busy = false
    private var executing = false
    private var jobs: [() -> Void] = []

    private init() {
        let thread = Thread { [self] in
            ready.lock(); loop = CFRunLoopGetCurrent(); ready.signal(); ready.unlock()
            timer = Timer.scheduledTimer(withTimeInterval:3, repeats:true) { [weak self] _ in self?.enqueue { [weak self] in self?.heartbeat() } }
            CFRunLoopRun()
        }
        self.thread = thread
        thread.name = "LogiLite.HID"
        ready.lock(); thread.start()
        while loop == nil { ready.wait() }
        ready.unlock()
    }
    private func enqueue(_ body: @escaping () -> Void) {
        CFRunLoopPerformBlock(loop!, CFRunLoopMode.defaultMode.rawValue) { [self] in
            jobs.append(body)
            guard !executing else { return }
            executing = true
            while !jobs.isEmpty { jobs.removeFirst()() }
            executing = false
        }
        CFRunLoopWakeUp(loop!)
    }
    func perform(_ command: [String: Any]) -> [String: Any] {
        let semaphore = DispatchSemaphore(value:0)
        var result: [String:Any] = [:]
        enqueue { [self] in
            busy = true
            // Enumeration must still identify exactly the retained endpoint.
            let candidates = HardwareService.endpoints().filter { $0.isDirectTarget }
            let retained = candidates.count == 1 && candidates.first?.registryID == session?.endpoint.registryID ? session : nil
            result = HardwareService().performOnWorker(command:command, session:retained)
            busy = false
            semaphore.signal()
        }
        semaphore.wait()
        return result
    }
    func configure(enabled: Bool, onPress: @escaping () -> Void, onStatus: @escaping (String?) -> Void, wait: Bool) {
        let semaphore = DispatchSemaphore(value:0)
        enqueue { [self] in
            busy = true
            delivery.invalidate(); delivery = TopDeliveryGate()
            self.onPress = onPress; self.onStatus = onStatus
            wanted = enabled
            if !enabled { restore() }
            busy = false
            if enabled { heartbeat() }
            semaphore.signal()
        }
        if wait { semaphore.wait() }
    }
    @discardableResult
    private func restore() -> Bool {
        delivery.invalidate()
        session?.onNotification = nil
        if diverted, let feature, let client = protocolClient, let session,
           HardwareService.endpoints().contains(where:{ $0.registryID == session.endpoint.registryID }) {
            session.deadline = Date().addingTimeInterval(2)
            do {
                try client.divert(feature, enabled:false)
                if let identity = session.endpoint.reconnectIdentity { ownedIdentities.remove(identity) }
            }
            catch {
                // Keep ownership and retry even with the window closed. A lost
                // acknowledgement must not abandon a still-diverted device.
                onStatus?("顶部按钮恢复失败，正在重试；也可关闭再开启鼠标电源")
                return false
            }
        }
        session = nil; feature = nil; protocolClient = nil
        diverted = false; tracker = TopButtonPressTracker()
        onStatus?(nil)
        return true
    }
    private func restoreReconnectedDevice() {
        let candidates = HardwareService.endpoints().filter { $0.isDirectTarget }
        guard candidates.count == 1, let endpoint = candidates.first,
              let identity = endpoint.reconnectIdentity, ownedIdentities.contains(identity) else { return }
        let connection = HIDSession(endpoint, deadline:Date().addingTimeInterval(10))
        guard connection.open() else { return }
        do {
            let client = TopButtonProtocol(exchange: { [weak connection] in connection?.exchange($0) })
            let index = try client.discover(), flags = try client.reporting(index)
            guard flags & 4 == 0 else { throw HIDTopError.message("请先退出接管顶部按钮的软件") }
            if flags & 1 != 0 { try client.divert(index, enabled:false) }
            ownedIdentities.remove(identity)
            onStatus?(nil)
        } catch { onStatus?(error.localizedDescription) }
    }
    private func heartbeat() {
        guard !busy else { return }
        busy = true; defer { busy = false }
        guard wanted else {
            if diverted { restore() }
            if session == nil && !ownedIdentities.isEmpty { restoreReconnectedDevice() }
            return
        }
        do {
            let candidates = HardwareService.endpoints().filter { $0.isDirectTarget }
            guard candidates.count == 1, let endpoint = candidates.first else {
                restore()
                throw HIDTopError.message(candidates.count > 1 ? "顶部按钮：请只连接一只目标鼠标" : "顶部按钮等待鼠标连接")
            }
            if session?.endpoint.registryID != endpoint.registryID {
                guard restore() else { return }
                delivery = TopDeliveryGate()
                let connection = HIDSession(endpoint, deadline:Date().addingTimeInterval(10))
                guard connection.open() else { throw HIDTopError.message(connection.lastError ?? "无法连接顶部按钮") }
                session = connection
                let client = TopButtonProtocol(exchange: { [weak connection] in connection?.exchange($0) })
                let index = try client.discover()
                let flags = try client.reporting(index)
                let previouslyOwned = endpoint.reconnectIdentity.map { ownedIdentities.contains($0) } ?? false
                guard flags & 4 == 0, flags & 1 == 0 || previouslyOwned else {
                    throw HIDTopError.message("顶部按钮已被其他软件接管，请退出 Options+ 后重连鼠标")
                }
                feature = index
                // Set ownership before writing so a lost acknowledgement still
                // attempts restoration instead of abandoning a diverted button.
                protocolClient = client; diverted = true
                if let identity = endpoint.reconnectIdentity { ownedIdentities.insert(identity) }
                try client.divert(index, enabled:true)
                protocolClient = client
                connection.onNotification = { [weak self] bytes in
                    guard let self, self.wanted, let feature = self.feature,
                          self.diverted, self.tracker.consume(bytes, feature:feature) else { return }
                    let callback = self.onPress, token = self.delivery
                    DispatchQueue.main.async {
                        guard token.isValid else { return }
                        callback?()
                    }
                }
            } else if session?.onNotification == nil {
                guard restore() else { return }
            } else if let session, let feature, let client = protocolClient {
                session.deadline = Date().addingTimeInterval(3)
                let flags = try client.reporting(feature)
                if flags & 4 != 0 { throw HIDTopError.message("Options+ 正在接管顶部按钮，请退出其后台") }
                if flags & 1 == 0 {
                    tracker = TopButtonPressTracker()
                    try client.divert(feature, enabled:true)
                }
                protocolClient = client
            }
            onStatus?(nil)
        } catch {
            restore()
            onStatus?(error.localizedDescription)
        }
    }
}

enum TopButtonHardware {
    static func configure(enabled: Bool, onPress: @escaping () -> Void, onStatus: @escaping (String?) -> Void, wait: Bool = false) {
        HIDWorker.shared.configure(enabled:enabled, onPress:onPress, onStatus:onStatus, wait:wait)
    }
}

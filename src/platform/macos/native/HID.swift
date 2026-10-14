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
    let deadline: Date
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
    private func endpoints() -> [HIDEndpoint] {
        let manager = IOHIDManagerCreate(kCFAllocatorDefault, IOOptionBits(0))
        IOHIDManagerSetDeviceMatching(manager, [kIOHIDVendorIDKey: 0x046d] as CFDictionary)
        // Matching enumerates metadata without requesting TCC or seizing input.
        let devices = IOHIDManagerCopyDevices(manager) as? Set<IOHIDDevice> ?? []
        return devices.map(HIDEndpoint.init).sorted { $0.registryID < $1.registryID }
    }
    func discover() -> [[String: Any]] { endpoints().map { $0.metadata } }
    func perform(command: [String: Any]) -> [String: Any] {
        let endpoints = endpoints()
        let devices = endpoints.map { $0.metadata }
        let candidates = endpoints.filter { $0.isDirectTarget }
        guard candidates.count == 1, let endpoint = candidates.first else {
            return ["ok": false, "devices": devices, "error": candidates.count > 1
                ? "发现多只目标鼠标，请只连接要测试的一只后刷新。"
                : "未找到提供 HID++ 长报告的 MX Anywhere 3 蓝牙端点。可先使用按键映射，唤醒鼠标后重试。"]
        }
        let session = HIDSession(endpoint, deadline: Date().addingTimeInterval(12))
        guard session.open() else { return ["ok": false, "devices": devices, "error": session.lastError ?? "无法打开设备"] }
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
            if let error = session.lastError { result["transport_error"] = error }
            return result
        }
    }
}

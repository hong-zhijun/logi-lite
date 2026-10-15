import Foundation

@main
struct HIDLayoutTests {
    static func descriptor(_ hex: String) -> Data {
        precondition(hex.count.isMultiple(of: 2))
        let bytes = Array(hex.utf8)
        return Data(stride(from: 0, to: bytes.count, by: 2).map { i in
            UInt8(String(bytes: bytes[i..<i + 2], encoding: .utf8)!, radix: 16)!
        })
    }

    static func testTopButtonProtocol() {
        var sent: [[UInt8]] = []
        var flags: UInt8 = 0
        let client = TopButtonProtocol { packet in
            sent.append(packet)
            var reply = packet
            let function = packet[3] >> 4
            if packet[2] == 0 {
                precondition(packet[4...5] == [0x1b, 4][0...1])
                reply[4] = 9
            } else {
                precondition(packet[2] == 9)
                switch function {
                case 0: reply[4] = 2
                case 1:
                    let info: [UInt8] = packet[4] == 0 ? [0, 0x52, 0, 0, 0x20] : [0, 0xc4, 0, 0x9d, 0x20]
                    reply.replaceSubrange(4..<9, with:info)
                case 2:
                    precondition(packet[4] == 0 && packet[5] == 0xc4)
                    reply[6] = flags
                case 3:
                    precondition(Array(packet[4..<9]) == [0, 0xc4, packet[6], 0, 0])
                    precondition(packet[6] == 3 || packet[6] == 2)
                    flags = packet[6] & 1
                default: preconditionFailure("Unexpected top protocol function")
                }
            }
            return reply
        }
        do {
            let feature = try client.discover()
            precondition(feature == 9 && sent.count == 4)
            let initialFlags = try client.reporting(feature)
            precondition(initialFlags == 0)
            try client.divert(feature, enabled:true)
            precondition(flags == 1)
            try client.divert(feature, enabled:false)
            precondition(flags == 0)
        } catch { preconditionFailure(error.localizedDescription) }
        precondition(sent.allSatisfy { $0[3] & 15 != 0 })
        for scenario in 0..<4 {
            let invalid = TopButtonProtocol { request in
                var reply = request
                if scenario == 0 { reply[3] ^= 1 }
                if scenario == 1 { reply[1] = 2 }
                if scenario == 2 { return Array(reply.prefix(6)) }
                if scenario == 3 { reply[2] = 0xff }
                return reply
            }
            do { _ = try invalid.request(9, 2, [0, 0xc4], needed:5); preconditionFailure("Accepted invalid packet") }
            catch { }
        }
        let refused = TopButtonProtocol { request in
            var reply = request
            if request[3] >> 4 == 2 { reply[6] = 0 }
            return reply
        }
        do { try refused.divert(9, enabled:true); preconditionFailure("Accepted unconfirmed diversion") }
        catch { }
        var tracker = TopButtonPressTracker()
        func event(_ controls: [UInt8]) -> [UInt8] { [0x11, 255, 9, 0] + controls + [UInt8](repeating:0, count:16-controls.count) }
        precondition(tracker.consume(event([0, 0xc4]), feature:9))
        precondition(!tracker.consume(event([0, 0xc4]), feature:9))
        precondition(!tracker.consume(event([0, 0x52, 0, 0xc4]), feature:9))
        for offset in 0..<4 {
            var invalid = event([])
            invalid[offset] = [0x10, 2, 8, 0x08][offset]
            precondition(!tracker.consume(invalid, feature:9) && tracker.pressed)
        }
        precondition(!tracker.consume(event([]), feature:9) && !tracker.pressed)
        precondition(tracker.consume(event([0, 0x52, 0, 0xc4]), feature:9))
        precondition(!tracker.consume(Array(event([]).prefix(11)), feature:9) && tracker.pressed)
    }

    static func main() {
        // Actual connected MX Anywhere3 BLE descriptor. Primary usage is Mouse;
        // HID++ is a second collection with 19 payload bytes and report ID 0x11.
        let actual = descriptor("05010902a10185020901a1009510750115002501050919012910810205011601f826ff07750c95020930093181061581257f75089501093881069501050c0a38028106c0c00643ff0a0202a101851175089513150026ff000902810009029100c0")
        let layout = HIDReportLayout(actual)
        precondition(layout.inputs[2] == 8)
        precondition(layout.inputs[17] == 20)
        precondition(layout.outputs[17] == 20)
        precondition(layout.supportsLongReport)

        // Report ID is a wire byte; report 0 has no ID prefix. Round partial
        // bytes up after accumulating each report's bit fields.
        let unnumbered = HIDReportLayout(descriptor("7501950381027501950281029102"))
        precondition(unnumbered.inputs[0] == 1)
        precondition(unnumbered.outputs[0] == 1)
        precondition(!unnumbered.supportsLongReport)

        // PUSH/POP restore Report ID, Report Size and Report Count together.
        let pushed = HIDReportLayout(descriptor("851175089513a4851295058102b481029102"))
        precondition(pushed.inputs[18] == 6)
        precondition(pushed.inputs[17] == 20)
        precondition(pushed.outputs[17] == 20)
        precondition(pushed.supportsLongReport)

        // A long item is opaque and must not overwrite global report fields.
        let longItem = HIDReportLayout(descriptor("851175089513fe030175019581029102"))
        precondition(longItem.supportsLongReport)

        // Truncated short/long items and unmatched POP must fail closed,
        // even after valid input/output fields have already been encountered.
        for suffix in ["76ff", "fe", "fe0301ff", "b4"] {
            let malformed = HIDReportLayout(actual + descriptor(suffix))
            precondition(malformed.inputs.isEmpty && malformed.outputs.isEmpty)
            precondition(!malformed.supportsLongReport)
        }
        precondition(!HIDReportLayout(Data()).supportsLongReport)
        precondition(!HIDReportLayout(descriptor("85117508951281029102")).supportsLongReport)
        testTopButtonProtocol()
        print("Offline HID descriptor and top-button protocol tests passed")
    }
}

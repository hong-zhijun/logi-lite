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
        print("Offline HID descriptor tests passed")
    }
}

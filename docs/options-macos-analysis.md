# Installed Options+ macOS analysis

Analyzed on 2026-10-14, read-only. Target supplied by the user: installed Logitech Options+. No installed files or Options+ settings were changed; no hardware commands were sent during this analysis. The REA active native target has been closed.

## Identity and retained evidence

| Evidence | Observation |
| --- | --- |
| `ev_90750216edd7a81431a754046a30d95626b3942a938ba1083ff702617a7adf74` | REA `inspect_plist`: `/Applications/logioptionsplus.app/Contents/Info.plist`, version `2.5.926888`, identifier `com.logi.optionsplus`, Electron `AtomApplication`, ASAR integrity declaration, minimum OS 12.0. |
| `ev_2ca160293d518419da19fc64721162ff041646062f1664cfd756fe067ade63b9` | REA `inspect_plist`: native agent Info.plist, version `2.5.926888`, identifier `com.logi.cp-dev-mgr`, `LSUIElement=true`, Bluetooth communication usage description. |
| `ev_ac55cfc923d4e38511653b3dc30f3e56801ba11971d1822e45d60b96cf30ffff` | REA `inspect_macho`: agent universal Mach-O contains x86_64 and arm64 slices; imports, exports and linked frameworks observed statically. |

The complete three REA records are retained in `options-rea-evidence.json`. Focused supplemental byte-string observations, with exact file offsets, are in `options-agent-static-strings.json`; those are a separate ordinary file-byte analysis, **not REA decompilation or execution evidence**.

Agent path: `/Library/Application Support/Logitech.localized/LogiOptionsPlus/logioptionsplus_agent.app/Contents/MacOS/logioptionsplus_agent`.

Agent SHA-256: `7d71273c6c3310edaebee729bf8f1f255c541f08651331bd6068dbd7a7ff3fc9`.

Frontend executable SHA-256, from REA open: `e8c5f21a241d014120ebea3d72c45b5caaced04391397c484ce25edc8a063ac5`.

## Observed input and device APIs

The agent imports `IOHIDManagerCreate`, `IOHIDManagerSetDeviceMatchingMultiple`, device arrival/removal callbacks, `IOHIDDeviceOpen`, `IOHIDDeviceCopyMatchingElements`, `IOHIDElementGetUsagePage`, `IOHIDElementGetUsage`, `IOHIDElementGetReportID`, `IOHIDElementGetReportSize`, `IOHIDDeviceGetReport`, `IOHIDDeviceSetReport`, `IOHIDDeviceRegisterInputReportCallback` and CFRunLoop scheduling APIs.

The same image imports `CGEventTapCreate`, `CGEventTapEnable`, `CGEventTapIsEnabled`, keyboard/mouse/scroll event creation, `CGEventPost`, `CGEventPostToPid`, `AXIsProcessTrustedWithOptions`, `IOHIDCheckAccess` and `IOHIDRequestAccess`. Linked frameworks include IOKit, CoreGraphics, ApplicationServices, CoreBluetooth and IOBluetooth.

The exported symbol `logi::hidio::hid_device_impl_osx::get_endpoint_data(IOHIDDevice*, IOHIDElement*)` is present at reported linked address `0x101a50714`. Arrival/removal and property-helper symbols are also exported. These observations establish available API paths, not their call order or that a particular mouse exercises every path.

Supplemental strings include:

| File offset | Observed ASCII string |
| --- | --- |
| 55542222 | `found report id %02x for usage %04x:%04x, output payload size %d` |
| 55542287 | `found report id %02x for the single collection, report size %d` |
| 55448632 | `-- Accessibility has not been granted, but input monitoring has.` |
| 55448698 | `-- We are already granted accessibility AND input monitoring. No need for user notifications.` |
| 55822076 | `Accessibility NOT GRANTED. Leaving 0x2121 high res scrolling to OOB.` |
| 56139676 | `SimInput: Accessibility NOT GRANTED.` |

Inference: Logitech implements both HID endpoint communication and OS-level event handling, and distinguishes accessibility from input monitoring. The collection/report diagnostic strings support discovery using descriptor elements rather than assuming a vendor usage is the device's primary usage. The OOB diagnostic supports preserving ordinary scrolling when accessibility is unavailable. Control flow has not been decompiled, so these are design guidance, not complete reconstruction.

## Observed feature families

The focused byte scan recovered these RTTI names. IDs here are part of names and **do not establish which features this connected mouse supports**.

| Feature family | Representative offset | Observed name |
| --- | --- | --- |
| Root | 52875166 | `IFeature0000Root` |
| Feature set | 52817296 | `IFeature0001FeatureSet` |
| Battery level | 54313512 | `IFeature1000BatteryUnifiedLevelStatus` |
| Battery voltage | 53101330 | `IFeature1001BatteryVoltage` |
| Unified battery | 53519008 | `IFeature1004UnifiedBattery` |
| Reprogrammable keys | 53177040 | `IFeature1B04SpecialKeys` |
| SmartShift | 53202550 | `IFeature2110SmartShift` |
| SmartShift with torque | 53202752 | `IFeature2111SmartShiftWithTunableTorque` |
| High resolution wheel | 52837520 | `IFeature2121HiResWheel` |
| Adjustable DPI | 53530054 | `IFeature2201AdjustableDPI` |

`1B04` names also include `GetCountResponse`, `GetCtrlIdInfoResponse`, `GetCtrlIdReportingResponse`, `GetCapabilitiesResponse`, `KeyReport`, `RawXYReport` and `RawWheelReport`. This supports capability/control discovery followed by report-driven key/gesture handling as an implementation strategy. It does not reveal packet byte layouts or prove that a particular control permits diversion.

## Guidance for an independent implementation

1. Use a native macOS transport built on IOHIDManager/IOHIDDevice and CFRunLoop. Filter Logitech devices, then inspect descriptor collections/elements for the HID++ endpoint. A mouse primary usage must not exclude its vendor collection.
2. Select the report ID and output length from the descriptor. The parent investigation independently observed the connected MX Anywhere3 BLE (`046d:b025`) with vendor collection `ff43:0202`, report ID `0x11`, 20-byte input/output reports. That observation comes from the parent's I/O Registry inspection, not these REA records. Do not assume USB's short-report availability on Bluetooth.
3. Obtain feature indexes dynamically through Root. Treat the families above as candidates, and handle missing features without aborting ordinary operation. Battery fallback should probe supported families instead of assuming `0x1000`.
4. For remapping, discover control IDs and reporting/capability state before changing diversion. Keep response matching separate from unsolicited reports. Do not hardcode feature indexes from another firmware/device.
5. Keep OS input mapping in a separate CGEventTap/injection layer. Check permissions, report actionable permission status, ignore self-generated events, re-enable disabled taps, and retain a usable standard-input path when specialized HID behavior is unavailable.
6. Test with Options+ exited by the user: competing agents may alter the same device settings or process the same events. This is a possible interference mechanism inferred from shared APIs, not an observed conflict in this analysis. No Options+ process was stopped here.

## Coverage and unknowns

Identity/version, architecture and API imports are answered by REA. The feature families and permission diagnostics are partially answered by exact byte strings. Connected-device feature support, CID values, actual request/response bytes, report framing choices and current permission state require the project's device diagnostics and the user's runtime test.

The frontend ASAR was identified but not analyzed; its full UI graph is unnecessary for the macOS input backend. Packaged `options_devices.json`, `guru_devices.json`, `DeviceCompatibility.json` and `defaults_control_osx.json` were parsed with JSON-aware reads; this bounded subset did not provide an MX Anywhere name-to-transport mapping. That negative result must not be generalized to the entire installation.

No native pseudocode or full control-flow analysis was performed. REA's configured Hopper provider was available, but static Mach-O inspection provided sufficient API evidence without importing the large agent into a disassembler. Ghidra and IDA were not configured; no installation or configuration changes were attempted. No credentials, user configuration or private runtime payloads were inspected.

# Mac validation — 2026-10-14

Host: Apple Silicon arm64, macOS 27.0.1 (26A434). Local development toolchain: Rust 1.97.1, Swift 6.3.2. Application is explicitly built for macOS 13.0+, with an ad-hoc signature.

## Observed hardware, read-only

The IORegistry descriptor and the new native backend both identify Logitech MX Anywhere 3, vendor 0x046d, product 0xb025, Bluetooth Low Energy. Primary usage is mouse (1:2); a second vendor collection ff43:0202 contains report ID 0x11 with 19 payload bytes / 20 total bytes for input and output. Descriptor parsing tests use this exact descriptor.

The new application's `--probe` successfully sent only HID++ read/discovery requests, using asynchronous output reports with completion and bounded response waiting. Observed responses:

- Identity: MX Anywhere 3, HID++ protocol 4.5, direct device index 255.
- Battery: 70%, discharging; a coarse level is also returned.
- DPI: 1000; device-supported range 200–4000 in steps of 50.
- Scroll: SmartShift, disengagement threshold 12.
- Battery, DPI, scroll and SmartShift capabilities reported available; no probe warnings.

Options+ agent remained running during this read test. Read success does not establish freedom from interference during remapping or settings writes. No software stopped the agent and no hardware setting was written.

## Verification boundary

19 Rust tests passed, along with both native Swift test suites. Rust tests cover shared protocol framing, correlation, timeouts, late replies, capabilities, DPI and scroll values, configuration, Mac modifier aliases and rejection of malformed native commands before hardware access. Native Swift tests cover virtual-key translation, swallowed down/up/drag pairs, the actual Bluetooth descriptor, malformed descriptors and HID global state.

Physical mouse-button mapping and settings writes were subsequently exercised by the user as recorded below. Permissions are requested only by a deliberate UI action; metadata/probe CLI paths never request TCC permissions.

The final bundled application passed `--smoke-test`: window created and visible, menu created, mapping inactive, and no configuration written. AppKit constraint activation order was corrected before this successful run. `codesign --verify --strict` and Cargo formatting / Clippy checks passed. Visual inspection through computer use was unavailable because the Mac became locked; no permission or lock setting was changed.

## User-reported manual results — 2026-10-14

These are the user's observations on the connected Bluetooth MX Anywhere 3, not an automated execution trace.

| Check | Result |
| --- | --- |
| Physical side-button shortcut mapping | Reported working |
| Middle button (pressing the scroll wheel) | Reported working |
| Disabling side buttons | Reported working |
| Mapping after closing the settings window | Reported working |
| Shortcut text retained after quit and reopen | Reported working |
| DPI changes | Reported working |
| Free / ratchet wheel modes | Reported working |
| Bluetooth disconnect and reconnect | Reported working |
| SmartShift threshold's physical effect | Inconclusive: user could not perceive a change |

Still unconfirmed: emergency/menu pause behavior, rapid/held-button edge cases, mapping state after reopening, sleep/wake, and SmartShift's actual automatic transition at different thresholds. The report does not establish other mouse models or OS versions.

SmartShift affects automatic transition from ratchet to free spin as the wheel is turned faster. Earlier suggested values 5 and 40 referred to raw hardware thresholds, not Options+ percentages. The updated Mac application uses the verified Options+ 0–100% scale; see `settings-ranges.md` and `options-smartshift-native.md`. Compare 0% and 100%, read back the percentage and mode, and turn the wheel slowly then quickly. Higher sensitivity should enter free spin more easily. Its sensitivity is separate from ratchet force, which the application preserves. A successful configuration readback does not itself prove the mechanical behavior.

## Range correction verification — 2026-10-14

The Mac UI and bridge now accept SmartShift sensitivity 0–100%, convert using the installed Options+ agent's formula, and report the inverse percentage. Raw threshold 50 is correctly classified as enabled SmartShift; only raw 255 with mode 2 is classified as locked ratchet. Hardware DPI continues to use device-supported values and steps.

`scripts/check-macos.sh` passed: formatting, clippy, 20 Rust tests and native input/HID tests. The new protocol fixture checks percentage endpoints, rounding/quantization examples, and an actual simulated raw-50 write/read transaction preserving torque. Invalid percentage commands are rejected before hardware access.

`scripts/build-macos.sh` passed, including strict ad-hoc signature verification. Updated `dist/Logi Lite.app` and `dist/Logi-Lite-macOS-arm64.zip`. Native UI smoke test passed (`windowVisible`, `menuCreated`: true; `mappingActive`, `configWritten`: false). The first sandboxed launch was denied by native GUI initialization; the same self-test completed with approved native app access. No hardware settings were written during this investigation or verification. Updated physical SmartShift endpoint behavior remains for the user to test.

## Built-in macOS actions — 2026-10-14

The Mac button fields now offer editable dropdowns with `打开 App` and `调度中心`. These are distinct persisted action strings and bypass shortcut injection. App resolves `com.apple.apps.launcher`, falling back to the older `com.apple.launchpad.launcher`; Mission Control resolves `com.apple.exposelauncher`. The application opens through public `NSWorkspace.openApplication`, queued outside the event-tap callback, and reports launch failures in the mapping status. The installed Options+ native launch paths and evidence are documented in `options-system-actions-native.md`.

Formatting, clippy, all 20 Rust tests and Swift input/HID checks passed. The input parser recognizes system actions separately from shortcuts, and native GUI-session resolution tests found both system apps. An initial Launch Services resolution test was blocked by the restricted sandbox; the explicit `--resolve-system-actions` integration check passed with native-session access. Strict signature verification and UI smoke check passed; config was not written and mapping was inactive during smoke verification. Updated app and zip rebuilt. Physical side-button activation of the new actions remains for user testing; no user's button configuration or macOS keyboard shortcuts were changed.

## Direction gestures — 2026-10-14

**Historical experiment: removed after the user's feedback that it was difficult to use.** Current software has no gesture UI or gesture runtime. Design, configuration and feedback are archived in [gesture-button-experiment.md](gesture-button-experiment.md).

Added opt-in `手势按钮` with a shared five-action configuration for click/up/down/left/right. Old version-1 configuration remains valid with default gesture settings. Direction callbacks fire only at release, once per press. Relative movement threshold defaults to 40 (application-defined 15–300). The first dominant direction locks; significant ambiguous diagonal movement suppresses click. Esc, other button/scroll activity, pause/configuration changes, sleep/session deactivation, a 10-second hold limit and lost-release detection cancel pending gestures. Canceled side-button drag events are converted to ordinary motion to avoid retaining movement suppression. Only transient relative deltas are retained; no mouse history is logged.

21 Rust tests passed, including old-config decoding, new-config roundtrip and recursive/invalid gesture rejection. Native tests passed for four directions, jitter, diagonal out-and-back, direction locking, duplicate release, overlapping buttons, cancellation, timeout and Swift config migration/roundtrip. Existing input pairing and HID tests also passed, with formatting and clippy clean. The expanded gesture panel passed native UI smoke verification (`gestureControlsOK`, `windowVisible`, `menuCreated`: true; `mappingActive`, `configWritten`: false). CUA selected the user's already-running old instance during visual inspection, so it did not establish the new panel's appearance; no old instance was quit or reconfigured. Build/signature checks passed. No actual mouse gestures were executed by the agent; physical direction detection, cursor suppression, distance tuning and reconnect/lost-release behavior remain for user testing.

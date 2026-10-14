# Options+ native system actions

Investigation date: 2026-10-14. Target: installed Logi Options+ 2.5.926888 arm64 slice of `/Library/Application Support/Logitech.localized/LogiOptionsPlus/logioptionsplus_agent.app/Contents/MacOS/logioptionsplus_agent`. Universal file SHA-256: `7d71273c6c3310edaebee729bf8f1f255c541f08651331bd6068dbd7a7ff3fc9`.

This is read-only offline static evidence, captured using Xcode llvm-objdump with exact `--macho --arch=arm64 --disassemble --dis-symname SYMBOL` scopes. No target execution, app launch, settings write, or Hopper session occurred. Raw targeted disassembly is retained in `analysis/options-system-actions/` (local analysis artifacts).

## Confirmed behavior

Options+ implements the built-in Apps/Launchpad and Mission Control actions by resolving and launching Apple's system application bundles through **public NSWorkspace APIs**. Neither observed action callback sends a user-configured keyboard shortcut.

| Action | System bundle identifier | Observed native path |
|---|---|---|
| Apps on Tahoe or later | `com.apple.apps.launcher` | `gestures::launch_pad` callback and `+[injector InjectLaunchpad]` |
| Launchpad on older macOS | `com.apple.launchpad.launcher` | same paths, selected when `is_tahoe_or_later()` returns false |
| Mission Control | `com.apple.exposelauncher` | `gestures::mission_control` callback and `+[injector InjectMissionControl]` |

Both resolve the URL via `NSWorkspace.sharedWorkspace.URLForApplicationWithBundleIdentifier:` and call `launchApplicationAtURL:options:configuration:error:` with options `0x10000`, empty NSDictionary, and null error pointer. The installed SDK NSWorkspace.h line 351 identifies `0x10000` as `NSWorkspaceLaunchAsync`; line 385 confirms the API is public but deprecated in favor of `openApplicationAtURL:configuration:completionHandler:`. An implementation using the modern API can preserve this mechanism without invoking a keyboard shortcut or depending on the Options+ agent.

## Call and data-flow evidence

- `+[injector InjectLaunchpad]` begins at **0x10170e070**. At 0x10170e07c it calls `is_tahoe_or_later`; 0x10170e080–0x10170e094 load the two bundle IDs and select Apps when true. At 0x10170e0a8 it resolves the chosen bundle URL; at 0x10170e0c4 it sets `w3 = 0x10000`; 0x10170e0d4 tail-calls `launchApplicationAtURL:options:configuration:error:`.
- `+[injector InjectMissionControl]` begins at **0x10170dea0**. At 0x10170debc it loads `com.apple.exposelauncher`; 0x10170dec0 resolves its URL; 0x10170dedc sets the launch options; 0x10170deec tail-calls the same launcher API.
- The primary `gestures::launch_pad()` at **0x101700290** posts its callback through `post_main_run_loop_job` at 0x1017002c8. Its callback begins at **0x10170589c**, independently performs the same OS-version bundle selection and NSWorkspace launch, with URL resolution at 0x1017058d4 and launch at 0x101705900.
- `gestures::mission_control()` begins at **0x101700360**, posts its callback at 0x101700398. Its callback begins at **0x1017059b4** (logging scaffolding precedes the actual launch); 0x101705ab4 loads `com.apple.exposelauncher`, 0x101705ab8 resolves its URL, and 0x101705adc launches it with options `0x10000`.
- `siminput::endpoint_impl::on_request()` contains direct action dispatch into these primary paths: **0x101259094** calls `gestures::launch_pad()`; **0x1012590d8** calls `gestures::mission_control()`.
- Dispatch at 0x101259064 reads the Input_System action from offset 0x10; 0x101259068 subtracts 8; its byte jump table resides at **0x10303e040** and uses base **0x10125908c** plus table byte × 4. Offline Mach-O segment mapping and reading those 22 bytes shows action **8** resolves to **0x10125908c** (Launchpad branch) and action **9** to **0x1012590d0** (Mission Control branch). This is concrete branch/call evidence, not a string-only attribution.

## Scope and limits

The system-action dispatcher-to-OS-launch chain is established statically. This pass did not trace every upstream side-button HID/profile/macro branch into that dispatcher, nor execute Options+ to observe the actual popup. No claim is made about repeated-click toggling, timing, permissions of specific button hooks, or alternate paths used by unexamined actions. The separate injector methods and dispatcher callbacks converge on the same observed NSWorkspace mechanism.

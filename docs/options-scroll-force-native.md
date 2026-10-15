# Options+ scroll force and charging status

Checked 2026-10-15 against the installed arm64 Options+ agent:

`/Library/Application Support/Logitech.localized/LogiOptionsPlus/logioptionsplus_agent.app/Contents/MacOS/logioptionsplus_agent`

## Scroll force

Options+ exposes **Scroll force** as an integer percentage from **1 to 100**. The
native settings manager clamps the value to this range and passes it directly to
the SmartShift-with-tunable-torque feature; there is no percentage conversion.
The bounded static evidence is `docs/evidence/options-smartshift-write-arm64.asm`,
addresses `0x1009c7114–0x1009c712c`: clamp the `+0x1c` scroll-force field to
1–100, then pass it as the third argument.

The HID++ 2.0 feature is `0x2111`. Its `GetRatchetControlMode` response is:

```text
byte 0: mode (1 free, 2 ratchet/SmartShift)
byte 1: SmartShift threshold (255 means locked ratchet)
byte 2: scroll force (1–100)
```

`SetRatchetControlMode` sends those three bytes unchanged. Logi Lite therefore
only exposes `scroll_force` when the 2111 capability bit is present, preserves
mode and threshold, writes byte 2, and reads all three bytes back before
reporting success. Unsupported devices are read-only for this setting.

Static instruction evidence from local `xcrun llvm-objdump` (ordinary shell
analysis, not REA native function Evidence) is retained in:

- `docs/evidence/options-torque-caps-arm64.asm`
- `docs/evidence/options-torque-get-arm64.asm`
- `docs/evidence/options-torque-set-arm64.asm`

REA inspected the same arm64 Mach-O (SHA-256
`7d71273c6c3310edaebee729bf8f1f255c541f08651331bd6068dbd7a7ff3fc9`), evidence
`ev_ac55cfc923d4e38511653b3dc30f3e56801ba11971d1822e45d60b96cf30ffff`.

## Charging

The existing HID++ battery readers already return a stable `status` string:
`discharging`, `recharging`, `almost_full`, `full`, `slow_recharge`,
`invalid_battery`, or `thermal_error`. Unified Battery (`0x1004`) and Battery
Status (`0x1000`) use the status byte directly; the Battery Voltage
(`0x1001`) path decodes its charging bit and slow-recharge flag. No fabricated
percentage is emitted when a protocol reports only a coarse level.

The read-only `battery` hardware action performs only protocol ping, device-name
validation and battery feature reads. It does not discover DPI or scroll
features, and rejects every write field before exchanging any HID++ packets.
This supports periodic charging updates without refreshing pending settings.
The response uses the regular `status.battery` and `target_verified` fields;
callers should merge only the battery into an existing complete status.

No real mouse settings were changed during this implementation. Packet fixtures
verify field preservation, unsupported-feature gating and mismatched readback;
mechanical ratchet feel and cable plug/unplug behavior remain user hardware
validation tasks. The REA session was closed after inventory.

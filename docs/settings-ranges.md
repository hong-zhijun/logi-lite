# MX Anywhere 3 setting ranges — 2026-10-14

The user's model is **MX Anywhere 3**, Bluetooth vendor/product `046d:b025`; do not substitute MX Anywhere 3S specifications.

| Value | Range / unit | Evidence |
| --- | --- | --- |
| Hardware DPI | 200–4000 DPI | Logitech's model-specific specification and this mouse's HID++ report agree. |
| Hardware DPI step | 50 DPI | Observed from this mouse's HID++ supported-DPI report, rather than assumed from the website. |
| Options+ pointer-speed control | 0–100%, step 1 in the ordinary control | The installed `app.min.js` pointer component declares min `0x0`, max `0x64`, and percentage text; its internal range is -1..1, displayed as `(value+1)*50`. |
| Options+ SmartShift sensitivity control | 0–100%, step 1 | The installed `app.min.js` sensitivity component declares min `0x0`, max `0x64`, step `0x1`, percentage text, and setting path `smartshift.sensitivity`. |
| Options+ SmartShift hardware output | Raw thresholds 4–50 | Installed native agent converts `50 − round(46 × sensitivity / 100)`; a locked ratchet uses 255. |
| Updated Logi Lite Mac SmartShift field | 0–100%, integer input | Uses the verified Options+ conversion and displays its inverse percentage on readback. |

The initial application UI exposed a 1–49 raw threshold and did not clearly explain the distinction between hardware units and Options+ user-facing percentages. The Mac input now uses 0–100% sensitivity. The previous `threshold >= 50` ratchet classification was corrected to `threshold == 255`: raw 50 is SmartShift sensitivity 0%. Pointer-speed percentage is not a linear rescaling of DPI; the installed code uses sensor min/max/default/step metadata and exponent/logarithm conversion. The DPI field continues to accept real DPI.

Hardware quantization means not every percentage is distinct: for example, requesting 75% writes raw 15 and reads back 76%, just as the native Options+ formula does. This software conversion is verified statically and with simulated protocol tests; the updated app's physical wheel behavior still requires user testing.

## Public sources

- [Logitech MX Anywhere 3 specification](https://support.logi.com/hc/en-us/articles/17743181662871-Specification-MX-Anywhere-3-Wireless-Mouse): Resolution 200–4000 DPI.
- [Logitech getting-started guide](https://support.logi.com/hc/en-us/articles/360053419773-Getting-Started-MX-ANYWHERE-3).
- [Official Options+ screenshot](https://cx-assets.logi.com/CC%20Graphics/MXAnywhere3/GSUpdate/GS_2.jpg): pointer speed 50%, SmartShift sensitivity 87%; screenshot confirms the units, not the endpoint ranges.

## Installed-artifact evidence and limits

Options+ version 2.5.926888. ASAR SHA-256: `570132e2682f7ffebd36d5f8d6c062fe2a12c143fa6807b053efac3ae6e34093`. `app.min.js` SHA-256: `17d7f7c9bfee9c987dbaf86ee73dd875b791718de38800b077d2016237fdeca0`; its content hash matches the archive's member integrity declaration.

REA retained the partial ASAR analysis as `ev_53b62b2ec2aaf81fc5c05820f78e35306726467df5ba51c2dd34d97d16e09382`, with summary view `ev_5d9def91611e8c1755806817ea8c71526a9f5bf8a4d9c649fd97802bedbb7587`. The broad analysis timed out on large preceding bundles before resolving `app.min.js`; it does **not** establish the two ranges. They were instead obtained by a bounded independent file-text analysis: verified ASAR member extraction, inert string-table parsing, the declared rotation checksum (`0xde4d5`, 32848 entries, 371 rotations), and literal property resolution. No JavaScript, frontend application, or backend agent was executed during this range analysis.

Focused observations, source identities and limitations are recorded in `options-range-observations.json`; public-source findings in `range-official-sources.md`. This is static configuration evidence, not observation of a running slider. Native conversion, endpoint semantics, bounded assembly evidence and REA provider limitations are recorded separately in [options-smartshift-native.md](options-smartshift-native.md).

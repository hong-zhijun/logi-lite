# MX Anywhere 3 ranges: official public evidence

Checked on 2026-10-14. This document concerns MX Anywhere 3 (including the Mac variant), not MX Anywhere 3S.

## Hardware DPI

Two accessible Logitech support specification pages independently state:

> DPI (Min/Max) — Resolution: 200 – 4,000 DPI

- https://support.logi.com/hc/en-us/articles/360053565493-Specification-MX-Anywhere-3-Wireless-Mouse
- https://support.logi.com/hc/en-us/articles/17743181662871-Specification-MX-Anywhere-3-Wireless-Mouse

The newer page covers MX Anywhere 3, MX Anywhere 3 for Mac, and MX Anywhere 3 for Business. Neither page specifies DPI step size or a mapping from the Options+ pointer-speed percentage to hardware DPI. That mapping must be established separately from the local software or device protocol.

## Options+ pointer speed and SmartShift display

The official getting-started guide contains an Options+ Point & Scroll screenshot:

- Guide: https://support.logi.com/hc/en-us/articles/360053419773-Getting-Started-MX-ANYWHERE-3
- Screenshot: https://cx-assets.logi.com/CC%20Graphics/MXAnywhere3/GSUpdate/GS_2.jpg

The screenshot visibly shows:

- `Pointer speed 50%`
- `Scroll force 44%`
- SmartShift `SENSITIVITY VALUE 87%`

This establishes percentage display in the official UI, but the screenshot does **not** display endpoint values. Do not infer exact slider limits or conversions to raw device values from this screenshot alone.

The guide states:

> You can also adjust the SmartShift sensitivity, which will change the speed required to automatically shift into free spinning.

It does not publish the numeric range, direction of the percentage slider, or conversion to the HID++ threshold byte.

## What SmartShift does

https://support.logi.com/hc/en-us/articles/360052340194-What-is-SmartShift-on-MX-Anywhere-3

> When SmartShift is enabled if you start scrolling faster while in Ratchet mode, your wheel automatically switches to free wheel mode (frictionless scrolling).

This official article provides no numeric range.

## Model difference

https://support.logi.com/hc/en-us/articles/13399995022743-What-is-the-difference-between-MX-Anywhere-3-and-MX-Anywhere-3S

> Better speed and more precise sensor performance, with the new 8000 DPI sensor.

The 8,000 DPI statement is about MX Anywhere 3S. It must not be used as the maximum for the user's ordinary MX Anywhere 3.

## Source caveats

Old product URLs with SKUs for ordinary MX Anywhere 3 currently return 404. The unqualified old `/en-us/products/mice/mx-anywhere-3.html` URL redirects to the 3S page; its current 8K marketing text is not evidence for the ordinary 3. Support articles above remain accessible and were read directly.

Pending separate local evidence: exact Options+ percentage-slider endpoints, SmartShift percentage direction and hardware conversion, and DPI step size.

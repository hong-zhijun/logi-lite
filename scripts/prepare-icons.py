#!/usr/bin/env python3
"""Derive macOS, UI and menu bar icons from the user-selected logo (Pillow)."""
from pathlib import Path
from PIL import Image, ImageFilter

ROOT = Path(__file__).resolve().parent.parent
ICONS = ROOT / 'src-tauri/icons'
source = Image.open(ICONS / 'logo-source.png').convert('RGBA')
resample = Image.Resampling.LANCZOS

# The selected flat app icon already includes its rounded tile and outer margin.
# Preserve the complete artwork rather than adding another tile around it.
source.resize((224, 224), resample).save(ROOT / 'ui/assets/logo.png')
icon = source.resize((1024, 1024), resample)
icon.save(ICONS / 'icon.png')
icon.save(ICONS / 'icon.icns', format='ICNS')

# Derive the menu bar template from the same selected logo. Ignore the light tile
# and its outer shadow; keep the dark mouse strokes and purple side button.
art = source.crop((round(source.width * .15), round(source.height * .22),
                   round(source.width * .86), round(source.height * .80)))
mask = Image.new('L', art.size)
mask.putdata([round(a * max(0, min(1, (215 - min(r, g, b)) / 65)))
              for r, g, b, a in art.getdata()])
mark = Image.new('RGBA', art.size)
mark.putalpha(mask)
mark = mark.crop(mask.getbbox())
mark.save(ICONS / 'tray-source.png')

def centered_mark(size, inset):
    image = Image.new('RGBA', (size, size))
    fitted = mark.copy()
    fitted.thumbnail((size - inset * 2, size - inset * 2), resample)
    image.alpha_composite(fitted, ((size - fitted.width) // 2, (size - fitted.height) // 2))
    return image

# macOS tints template icons itself; use only the logo's alpha and strengthen its
# strokes slightly so the silhouette stays clear at 22 logical pixels.
alpha = centered_mark(88, 8).getchannel('A').filter(ImageFilter.MaxFilter(3))
tray = Image.new('RGBA', (88, 88), (0, 0, 0, 0))
tray.putalpha(alpha)
tray.resize((44, 44), resample).save(ICONS / 'tray.png')
print('Updated UI logo, Dock icon and menu bar template icon.')

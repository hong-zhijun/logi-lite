#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
if [[ "$(uname -s)" != Darwin ]]; then
    echo 'This script needs macOS and Xcode Command Line Tools.' >&2
    exit 1
fi
export MACOSX_DEPLOYMENT_TARGET=13.0
cargo build --release --locked --manifest-path src-tauri/Cargo.toml
APP="$ROOT/dist/Logi Lite.app"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "$ROOT/target/release/LogiLite" "$APP/Contents/MacOS/LogiLite.next"
mv "$APP/Contents/MacOS/LogiLite.next" "$APP/Contents/MacOS/LogiLite"
cp src-tauri/icons/icon.png "$APP/Contents/Resources/app-logo.png"
python3 - "$APP/Contents/Info.plist" <<'PY'
import hashlib, pathlib, plistlib, shutil, sys
icon_source = pathlib.Path('src-tauri/icons/icon.icns')
icon_name = 'LogiLite-' + hashlib.sha256(icon_source.read_bytes()).hexdigest()[:12] + '.icns'
shutil.copyfile(icon_source, pathlib.Path(sys.argv[1]).parent / 'Resources' / icon_name)
value = {
 'CFBundleExecutable':'LogiLite', 'CFBundleIdentifier':'io.github.hong-zhijun.logi-lite',
 'CFBundleName':'Logi Lite', 'CFBundleDisplayName':'Logi Lite', 'CFBundlePackageType':'APPL',
 'CFBundleShortVersionString':'0.2.0', 'CFBundleVersion':'2',
 'LSMinimumSystemVersion':'13.0', 'NSHighResolutionCapable':True,
 'CFBundleIconFile':icon_name, 'LSUIElement':True,
 'NSBluetoothAlwaysUsageDescription':'Read the connected Logitech mouse battery, DPI and scroll settings.'
}
with open(sys.argv[1], 'wb') as f: plistlib.dump(value, f)
PY
printf 'APPL????' > "$APP/Contents/PkgInfo"
/usr/bin/codesign --force --sign - --identifier io.github.hong-zhijun.logi-lite "$APP"
/usr/bin/codesign --verify --strict "$APP"
echo "Built: $APP"

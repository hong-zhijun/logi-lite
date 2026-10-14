#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo fmt --all --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
node --check ui/app.js
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked -- -D warnings
cargo build --lib --locked
mkdir -p target/swift-module-cache
xcrun swiftc -target "$(uname -m)-apple-macosx13.0" \
    -module-cache-path target/swift-module-cache \
    src/platform/macos/native/Input.swift src/platform/macos/native/InputTests.swift \
    -o target/logilite-input-tests
target/logilite-input-tests
xcrun swiftc -target "$(uname -m)-apple-macosx13.0" \
    -module-cache-path target/swift-module-cache \
    src/platform/macos/native/MappingActivation.swift src/platform/macos/native/MappingActivationTests.swift \
    -o target/logilite-activation-tests
target/logilite-activation-tests
xcrun swiftc -target "$(uname -m)-apple-macosx13.0" \
    -module-cache-path target/swift-module-cache \
    src/platform/macos/native/HID.swift src/platform/macos/native/HIDTests.swift \
    -L target/debug -llogi_lite -framework IOKit \
    -o target/logilite-hid-tests
target/logilite-hid-tests

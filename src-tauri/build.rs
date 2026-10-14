use std::{env, path::PathBuf, process::Command};

fn main() {
    if env::var("CARGO_CFG_TARGET_OS").unwrap() == "macos" {
        let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("..");
        let native = root.join("src/platform/macos/native");
        let out = PathBuf::from(env::var("OUT_DIR").unwrap());
        let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
        let triple = format!(
            "{}-apple-macosx13.0",
            if arch == "aarch64" { "arm64" } else { "x86_64" }
        );
        let mut command = Command::new("xcrun");
        command.args([
            "swiftc",
            "-target",
            &triple,
            "-swift-version",
            "5",
            "-O",
            "-whole-module-optimization",
            "-emit-library",
            "-static",
            "-module-name",
            "LogiLiteNative",
            "-module-cache-path",
        ]);
        command.arg(out.join("swift-cache"));
        for name in [
            "Input.swift",
            "HID.swift",
            "MappingActivation.swift",
            "Runtime.swift",
        ] {
            println!("cargo:rerun-if-changed={}", native.join(name).display());
            command.arg(native.join(name));
        }
        assert!(
            command
                .arg("-o")
                .arg(out.join("libLogiLiteNative.a"))
                .status()
                .unwrap()
                .success(),
            "Swift backend compilation failed"
        );
        println!("cargo:rustc-link-search=native={}", out.display());
        println!("cargo:rustc-link-lib=static=LogiLiteNative");
        let info = Command::new("xcrun")
            .args(["swiftc", "-print-target-info"])
            .output()
            .unwrap();
        let info: serde_json::Value = serde_json::from_slice(&info.stdout).unwrap();
        for path in info["paths"]["runtimeLibraryPaths"].as_array().unwrap() {
            println!("cargo:rustc-link-search=native={}", path.as_str().unwrap());
        }
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/lib/swift");
        for framework in ["AppKit", "IOKit", "ApplicationServices"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
    }
    tauri_build::build();
}

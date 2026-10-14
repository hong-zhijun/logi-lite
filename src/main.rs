#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
use logi_lite::platform::windows::app;

fn main() {
    #[cfg(windows)]
    if let Some(path) = std::env::args().skip_while(|s| s != "--diagnose").nth(1) {
        let result = logi_lite::devices::diagnostic_json()
            .and_then(|data| std::fs::write(&path, data).map_err(|e| e.to_string()));
        if result.is_err() {
            std::process::exit(1);
        }
        return;
    }
    #[cfg(windows)]
    if let Err(error) = app::run() {
        if let Some(path) = std::env::args().skip_while(|s| s != "--smoke-test").nth(1) {
            let _ = std::fs::write(
                path,
                serde_json::json!({"success":false,"error":error}).to_string(),
            );
        } else {
            app::show_error(&error);
        }
        std::process::exit(1);
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::process::CommandExt;
        let executable = std::env::current_exe().unwrap_or_default();
        let root = executable
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent());
        let native = root.map(|root| root.join("dist/Logi Lite.app/Contents/MacOS/LogiLite"));
        if let Some(native) = native.filter(|path| path.is_file()) {
            let error = std::process::Command::new(native)
                .args(std::env::args_os().skip(1))
                .exec();
            eprintln!("Cannot launch Logi Lite: {error}");
        } else {
            eprintln!("请先运行 ./scripts/build-macos.sh，然后打开 dist/Logi Lite.app");
        }
        std::process::exit(1);
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    eprintln!("Logi Lite 0.1 currently supports Windows only.");
}

#![windows_subsystem = "windows"]

#[cfg(windows)]
mod app;
#[cfg(windows)]
mod worker;

fn main() {
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
    #[cfg(not(windows))]
    eprintln!("Logi Lite 0.1 currently supports Windows only.");
}

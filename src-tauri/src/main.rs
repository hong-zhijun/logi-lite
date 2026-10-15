#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde_json::{Value, json};
use std::sync::Mutex;
#[cfg(target_os = "macos")]
use std::ffi::{CStr, CString, c_char};
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn ll_desktop_request(json: *const c_char) -> *mut c_char;
    fn ll_desktop_hardware(json: *const c_char) -> *mut c_char;
    fn ll_desktop_free(pointer: *mut c_char);
}
struct RuntimeFlags {
    smoke: bool,
    hardware: Mutex<()>,
}

#[cfg(target_os = "macos")]
fn native(request: Value, hardware: bool) -> Result<Value, String> {
    let input = CString::new(request.to_string()).map_err(|e| e.to_string())?;
    // Both functions synchronously borrow the JSON and return a malloc-owned result.
    unsafe {
        let pointer = if hardware {
            ll_desktop_hardware(input.as_ptr())
        } else {
            ll_desktop_request(input.as_ptr())
        };
        if pointer.is_null() {
            return Err("Native backend returned no result".into());
        }
        let result =
            serde_json::from_slice(CStr::from_ptr(pointer).to_bytes()).map_err(|e| e.to_string());
        ll_desktop_free(pointer);
        result
    }
}

#[tauri::command]
fn window_minimize(window: tauri::WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|error| error.to_string())
}

#[tauri::command]
fn window_toggle_maximize(window: tauri::WebviewWindow) -> Result<bool, String> {
    let maximized = window.is_maximized().map_err(|error| error.to_string())?;
    if maximized { window.unmaximize().map_err(|error| error.to_string())?; }
    else { window.maximize().map_err(|error| error.to_string())?; }
    Ok(!maximized)
}

#[tauri::command]
fn window_close(window: tauri::WebviewWindow) -> Result<(), String> {
    // Keep the process and input mapping alive; the tray menu can reopen it.
    window.hide().map_err(|error| error.to_string())
}

#[cfg(windows)]
fn native(request: Value, hardware: bool) -> Result<Value, String> {
    let result = logi_lite::platform::windows::desktop::request(request, hardware);
    if result["ok"] == false { Err(result["error"].as_str().unwrap_or("Windows 后端失败").to_string()) } else { Ok(result) }
}

#[tauri::command]
async fn runtime(app: tauri::AppHandle, request: Value) -> Result<Value, String> {
    // Webview callers cannot reinitialize or select self-test mode.
    if request["action"] == "init" {
        return Err("Runtime is already initialized".into());
    }
    let (tx, rx) = std::sync::mpsc::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(native(request, false));
    })
    .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || rx.recv().map_err(|e| e.to_string())?)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn hardware(app: tauri::AppHandle, request: Value) -> Result<Value, String> {
    if app.state::<RuntimeFlags>().smoke {
        return Err("Self-test does not communicate with the mouse".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<RuntimeFlags>();
        let _guard = state.hardware.lock().map_err(|e| e.to_string())?;
        native(request, true)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn smoke_report(app: tauri::AppHandle, mut report: Value) -> Result<(), String> {
    if !app.state::<RuntimeFlags>().smoke {
        return Err("Not in self-test mode".into());
    }
    let visible = app
        .get_webview_window("main")
        .and_then(|window| window.is_visible().ok())
        .unwrap_or(false);
    report["windowVisible"] = json!(visible);
    report["ok"] = json!(report["ok"] == true && visible);
    let args: Vec<_> = std::env::args().collect();
    if let Some(i) = args.iter().position(|s| s == "--smoke-test") {
        let path = args.get(i + 1).ok_or("Missing self-test path")?;
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        app.exit(if report["ok"] == true { 0 } else { 1 });
    }
    Ok(())
}

fn main() {
    #[cfg(target_os = "macos")]
    // Keep the Rust C bridge visible to the statically linked Swift adapter.
    {
        std::hint::black_box(logi_lite::platform::macos::bridge::logilite_hardware as *const ());
        std::hint::black_box(logi_lite::platform::macos::bridge::logilite_load_config as *const ());
        std::hint::black_box(logi_lite::platform::macos::bridge::logilite_save_config as *const ());
        std::hint::black_box(logi_lite::platform::macos::bridge::logilite_free_string as *const ());
    }
    let args: Vec<_> = std::env::args().collect();
    for (flag, hardware) in [("--probe", true), ("--diagnose", false)] {
        if let Some(i) = args.iter().position(|s| s == flag) {
            let path = args.get(i + 1).expect("Missing output path");
            let result = native(
                json!({"action":if hardware {"probe"} else {"metadata"}}),
                hardware,
            )
            .unwrap();
            std::fs::write(path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
            return;
        }
    }
    let smoke = args.iter().any(|s| s == "--smoke-test");
    // One input hook owner per user. Self-tests never acquire or mutate user state.
    let _instance_lock = if smoke {
        None
    } else {
        let path = logi_lite::config::config_path()
            .expect("Cannot locate config directory")
            .with_file_name("desktop.lock");
        std::fs::create_dir_all(path.parent().unwrap()).expect("Cannot create config directory");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)
            .expect("Cannot open instance lock");
        if file.try_lock().is_err() {
            eprintln!("Logi Lite is already running; open it from the menu bar.");
            return;
        }
        Some(file)
    };
    tauri::Builder::default()
        .manage(RuntimeFlags {
            smoke,
            hardware: Mutex::new(()),
        })
        .invoke_handler(tauri::generate_handler![runtime, hardware, smoke_report, window_minimize, window_toggle_maximize, window_close])
        .setup(move |app| {
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main") {
                window.set_decorations(false).map_err(std::io::Error::other)?;
            }
            native(json!({"action":"init", "smoke":smoke}), false)
                .map_err(std::io::Error::other)?;
            let open = MenuItem::with_id(app, "open", "打开 Logi Lite", true, None::<&str>)?;
            let toggle = MenuItem::with_id(app, "toggle", "启用 / 暂停映射", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &toggle, &quit])?;
            TrayIconBuilder::new()
                .icon(tauri::image::Image::from_bytes(include_bytes!(
                    "../icons/tray.png"
                ))?)
                .icon_as_template(true)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "toggle" => {
                        let app2 = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            if let Ok(result) = native(json!({"action":"toggle"}), false)
                                && result["ok"] == false
                                && let Some(w) = app2.get_webview_window("main")
                            {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        });
                    }
                    "quit" => {
                        let app2 = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            let _ = native(json!({"action":"shutdown"}), false);
                            app2.exit(0);
                        });
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("Cannot build Logi Lite")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let _ = native(json!({"action":"shutdown"}), false);
                let _ = app;
            }
        });
}

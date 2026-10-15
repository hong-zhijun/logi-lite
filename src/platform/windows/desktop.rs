//! Tauri-facing Windows backend. It reuses the tested HID++ and input modules
//! without bringing the legacy Win32 settings window into the desktop app.
use crate::{
    config,
    hidpp::{self, Client, DeviceStatus, ScrollMode},
    platform::windows::{devices, input},
};
use serde_json::{Value, json};
use std::sync::{Mutex, OnceLock};

struct RuntimeState {
    config: config::Config,
    initialized: bool,
    smoke: bool,
}

static STATE: OnceLock<Mutex<RuntimeState>> = OnceLock::new();

fn state() -> &'static Mutex<RuntimeState> {
    STATE.get_or_init(|| {
        Mutex::new(RuntimeState {
            config: config::Config::default(),
            initialized: false,
            smoke: false,
        })
    })
}

fn response(result: Result<Value, String>) -> Value {
    match result {
        Ok(mut value) => {
            if value.get("ok").is_none() {
                value["ok"] = json!(true);
            }
            value
        }
        Err(error) => json!({"ok": false, "error": error}),
    }
}

pub fn request(request: Value, hardware: bool) -> Value {
    response((|| {
        let action = request
            .get("action")
            .and_then(Value::as_str)
            .ok_or("缺少 action")?;
        if hardware && !matches!(action, "probe" | "apply") {
            return Err("Windows 硬件后端不支持该操作".into());
        }
        match action {
            "init" => init(
                request
                    .get("smoke")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
            "state" => current_state(),
            "toggle" => toggle(),
            "save" => save(request.get("config").cloned().ok_or("缺少 config")?),
            "metadata" => metadata(),
            "probe" => probe(),
            "apply" => apply(&request),
            "export" => export(
                request
                    .get("hardware")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            ),
            "settings" => Ok(json!({"ok":true})),
            "shutdown" => shutdown(),
            _ => Err(format!("未知操作：{action}")),
        }
    })())
}

fn init(smoke: bool) -> Result<Value, String> {
    let mut runtime = state().lock().map_err(|_| "Windows 后端状态不可用")?;
    if runtime.initialized {
        return Err("Runtime 已初始化".into());
    }
    runtime.smoke = smoke;
    runtime.config = config::load().unwrap_or_default();
    runtime.initialized = true;
    if !smoke {
        input::set_config(&runtime.config)?;
        input::install()?;
    }
    current_state_locked(&runtime)
}

fn current_state() -> Result<Value, String> {
    let runtime = state().lock().map_err(|_| "Windows 后端状态不可用")?;
    current_state_locked(&runtime)
}

fn current_state_locked(runtime: &RuntimeState) -> Result<Value, String> {
    Ok(json!({
        "ok": true,
        "config": runtime.config,
        "active": runtime.config.enabled && !input::is_paused(),
        "inputStatus": input::stats(),
        "smoke": runtime.smoke,
        "appIconLoaded": true,
        "permissions": {"accessibility": true, "inputMonitoring": true, "postEvents": true}
    }))
}

fn toggle() -> Result<Value, String> {
    let mut runtime = state().lock().map_err(|_| "Windows 后端状态不可用")?;
    runtime.config.enabled = !runtime.config.enabled;
    config::save(&runtime.config)?;
    input::set_config(&runtime.config)?;
    current_state_locked(&runtime)
}

fn save(value: Value) -> Result<Value, String> {
    let config: config::Config =
        serde_json::from_value(value).map_err(|error| format!("配置格式错误：{error}"))?;
    config::validate(&config)?;
    let mut runtime = state().lock().map_err(|_| "Windows 后端状态不可用")?;
    config::save(&config)?;
    input::set_config(&config)?;
    runtime.config = config;
    current_state_locked(&runtime)
}

fn metadata() -> Result<Value, String> {
    let devices = devices::discover()?;
    let entries: Vec<Value> = devices
        .into_iter()
        .map(|device| {
            let candidate = device.is_hidpp_candidate();
            json!({
                "registry_id": device.path,
                "product": device.product,
                "manufacturer": device.manufacturer,
                "vid": device.vid,
                "pid": device.pid,
                "usage_page": device.usage_page,
                "usage": device.usage,
                "hidpp_candidate": candidate,
                "input_report_bytes": device.input_report_bytes,
                "output_report_bytes": device.output_report_bytes,
                "feature_report_bytes": device.feature_report_bytes
            })
        })
        .collect();
    Ok(json!({"ok":true,"devices":entries}))
}

struct Target {
    client: Client<devices::HidTransport>,
    status: DeviceStatus,
}

fn target() -> Result<Target, String> {
    let endpoints = devices::discover()?;
    for endpoint in endpoints
        .iter()
        .filter(|item| item.is_hidpp_candidate() && item.output_report_bytes.unwrap_or(0) >= 20)
    {
        for index in endpoint.candidate_indices() {
            let transport = match devices::HidTransport::open(endpoint) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let mut client = Client::new(transport, index);
            let status = match client.probe() {
                Ok(value) => value,
                Err(_) => continue,
            };
            if is_target(&status) {
                return Ok(Target { client, status });
            }
        }
    }
    Err("未识别到可通信的 MX Anywhere 3 系列鼠标".into())
}

fn is_target(status: &DeviceStatus) -> bool {
    status.name.as_deref().is_some_and(|name| {
        name.to_ascii_lowercase()
            .replace([' ', '-', '_'], "")
            .starts_with("mxanywhere3")
    })
}

fn probe() -> Result<Value, String> {
    let target = target()?;
    Ok(json!({"ok":true,"target_verified":true,"status":target.status}))
}

fn apply(request: &Value) -> Result<Value, String> {
    let mut target = target()?;
    let dpi = request
        .get("dpi")
        .and_then(Value::as_u64)
        .map(|value| u16::try_from(value).map_err(|_| "DPI 数值无效".to_string()))
        .transpose()?;
    let scroll = request
        .get("scroll")
        .and_then(Value::as_str)
        .map(|value| {
            serde_json::from_value::<ScrollMode>(json!(value)).map_err(|error| error.to_string())
        })
        .transpose()?;
    let sensitivity = request
        .get("sensitivity")
        .and_then(Value::as_u64)
        .map(|value| u8::try_from(value).map_err(|_| "SmartShift 灵敏度无效".to_string()))
        .transpose()?;
    if sensitivity.is_some() && scroll != Some(ScrollMode::SmartShift) {
        return Err("灵敏度只适用于 SmartShift".into());
    }
    if let Some(value) = dpi {
        target.client.set_dpi(value)?;
    }
    if let Some(mode) = scroll {
        let threshold = sensitivity
            .map(hidpp::smartshift_threshold)
            .transpose()?
            .unwrap_or_else(|| {
                target
                    .status
                    .scroll
                    .as_ref()
                    .map(|item| item.threshold)
                    .unwrap_or(50)
            });
        target.client.set_scroll(mode, threshold)?;
    }
    let status = target.client.probe()?;
    if !is_target(&status) {
        return Err("回读设备身份不匹配".into());
    }
    if dpi.is_some_and(|value| status.dpi != Some(value)) {
        return Err("DPI 回读不一致".into());
    }
    if scroll.is_some_and(|mode| status.scroll.as_ref().is_none_or(|item| item.mode != mode)) {
        return Err("滚轮模式回读不一致".into());
    }
    Ok(json!({"ok":true,"target_verified":true,"status":status}))
}

fn export(hardware: Value) -> Result<Value, String> {
    let path = config::config_path()?.with_file_name("diagnostic.json");
    let payload =
        json!({"app":"Logi Lite","platform":"windows","hardware":hardware,"input":input::stats()});
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&payload).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    Ok(json!({"ok":true,"path":path}))
}

fn shutdown() -> Result<Value, String> {
    input::uninstall()?;
    if let Ok(mut runtime) = state().lock() {
        runtime.initialized = false;
    }
    Ok(json!({"ok":true}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_matching_is_strict() {
        let make = |name: Option<&str>| DeviceStatus {
            device_index: 255,
            protocol: "4.5".into(),
            name: name.map(str::to_owned),
            battery: None,
            dpi: None,
            dpi_choices: vec![],
            dpi_ranges: vec![],
            scroll: None,
            capabilities: Default::default(),
            warnings: vec![],
        };
        assert!(is_target(&make(Some("MX Anywhere 3"))));
        assert!(is_target(&make(Some("MX Anywhere 3S"))));
        assert!(!is_target(&make(Some("MX Master 3"))));
        assert!(!is_target(&make(None)));
    }

    #[test]
    fn invalid_hardware_commands_are_rejected_before_device_access() {
        for command in [
            json!({"action":"apply","sensitivity":101}),
            json!({"action":"apply","sensitivity":50}),
            json!({"action":"apply","scroll":"ratchet","sensitivity":50}),
            json!({"action":"apply","device_index":0}),
        ] {
            let result = super::request(command, true);
            assert_eq!(result["ok"], false);
        }
    }
}

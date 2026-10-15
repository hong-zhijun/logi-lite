//! Tauri-facing Windows backend. It reuses the tested HID++ and input modules
//! without bringing the legacy Win32 settings window into the desktop app.
use crate::{
    config,
    hidpp::{self, Client, DeviceStatus, ScrollMode},
    platform::windows::{devices, input},
};
use serde_json::{Value, json};
use std::{
    sync::{Mutex, OnceLock, mpsc},
    time::Duration,
};

struct RuntimeState {
    config: config::Config,
    initialized: bool,
    smoke: bool,
}

static STATE: OnceLock<Mutex<RuntimeState>> = OnceLock::new();
static TOP_TX: OnceLock<mpsc::Sender<TopCommand>> = OnceLock::new();

enum TopCommand {
    Configure { enabled: bool, action: String },
    Stop,
}

struct TopSession {
    transport: Option<devices::HidTransport>,
    feature: u8,
    device_index: u8,
    pressed: bool,
}

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

fn top_sender() -> &'static mpsc::Sender<TopCommand> {
    TOP_TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<TopCommand>();
        std::thread::Builder::new()
            .name("logi-top-button".into())
            .spawn(move || top_loop(rx))
            .expect("cannot start top button worker");
        tx
    })
}

fn configure_top(config: &config::Config, smoke: bool) {
    if smoke {
        return;
    }
    let enabled = config.enabled && config.top.trim() != "默认";
    let _ = top_sender().send(TopCommand::Configure {
        enabled,
        action: config.top.clone(),
    });
}

fn top_loop(rx: mpsc::Receiver<TopCommand>) {
    let mut session: Option<TopSession> = None;
    let mut action = String::new();
    loop {
        if let Ok(command) = rx.recv_timeout(Duration::from_millis(100)) {
            match command {
                TopCommand::Stop => {
                    restore_top(&mut session);
                    break;
                }
                TopCommand::Configure {
                    enabled,
                    action: next,
                } => {
                    restore_top(&mut session);
                    action = next;
                    if enabled {
                        session = connect_top();
                    }
                }
            }
        }
        let Some(current) = session.as_mut() else {
            continue;
        };
        let report = match current
            .transport
            .as_mut()
            .and_then(|transport| transport.read_report(Duration::from_millis(80)).ok())
        {
            Some(value) => value,
            None => continue,
        };
        if top_pressed(&report, current.device_index, current.feature) && !current.pressed {
            current.pressed = true;
            let _ = input::trigger_action(&action);
        } else if !top_pressed(&report, current.device_index, current.feature) {
            current.pressed = false;
        }
    }
}

fn connect_top() -> Option<TopSession> {
    let endpoints = devices::discover().ok()?;
    for endpoint in endpoints
        .iter()
        .filter(|item| item.is_hidpp_candidate() && item.output_report_bytes.unwrap_or(0) >= 20)
    {
        for device_index in endpoint.candidate_indices() {
            let transport = devices::HidTransport::open(endpoint).ok()?;
            let mut client = Client::new(transport, device_index);
            let status = match client.probe() {
                Ok(value) => value,
                Err(_) => continue,
            };
            if !is_target(&status) {
                continue;
            }
            let root = client.command(0, 0, &[0x1b, 0x04], 3).ok()?;
            let feature = *root.first()?;
            if feature == 0 {
                continue;
            }
            let count = client.command(feature, 0, &[], 1).ok()?.first().copied()?;
            let mut supported = false;
            for index in 0..count {
                let info = client.command(feature, 1, &[index], 5).ok()?;
                if u16::from_be_bytes([info[0], info[1]]) == 0x00c4 && info[4] & 0x20 != 0 {
                    supported = true;
                    break;
                }
            }
            if !supported {
                continue;
            }
            let reporting = client.command(feature, 2, &[0, 0xc4], 5).ok()?;
            if reporting[2] & 4 != 0 {
                continue;
            }
            if reporting[2] & 1 == 0 {
                let _ = client.command(feature, 3, &[0, 0xc4, 3, 0, 0], 2).ok()?;
            }
            return Some(TopSession {
                transport: Some(client.into_transport()),
                feature,
                device_index,
                pressed: false,
            });
        }
    }
    None
}

fn restore_top(session: &mut Option<TopSession>) {
    if let Some(mut current) = session.take() {
        let Some(transport) = current.transport.take() else {
            return;
        };
        let mut client = Client::new(transport, current.device_index);
        let _ = client.command(current.feature, 3, &[0, 0xc4, 2, 0, 0], 2);
        // The transport is dropped after best-effort restoration.
    }
}

fn top_pressed(report: &[u8], device_index: u8, feature: u8) -> bool {
    if report.len() < 12
        || report[0] != 0x11
        || !(report[1] == device_index || (device_index == 255 && report[1] == 0))
        || report[2] != feature
        || report[3] != 0
    {
        return false;
    }
    (4..12)
        .step_by(2)
        .any(|index| u16::from_be_bytes([report[index], report[index + 1]]) == 0x00c4)
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
    configure_top(&runtime.config, smoke);
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
        "platform": "windows",
        "appIconLoaded": true,
        "permissions": {"accessibility": true, "inputMonitoring": true, "postEvents": true}
    }))
}

fn toggle() -> Result<Value, String> {
    let mut runtime = state().lock().map_err(|_| "Windows 后端状态不可用")?;
    runtime.config.enabled = !runtime.config.enabled;
    config::save(&runtime.config)?;
    input::set_config(&runtime.config)?;
    configure_top(&runtime.config, runtime.smoke);
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
    configure_top(&runtime.config, runtime.smoke);
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
    let scroll_force = request
        .get("scroll_force")
        .and_then(Value::as_u64)
        .map(|value| u8::try_from(value).map_err(|_| "滚动力度无效".to_string()))
        .transpose()?;
    if sensitivity.is_some() && scroll != Some(ScrollMode::SmartShift) {
        return Err("灵敏度只适用于 SmartShift".into());
    }
    if scroll_force.is_some() && !target.status.capabilities.scroll_force {
        return Err("设备未确认支持滚动力度设置".into());
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
    if let Some(force) = scroll_force {
        target.client.set_scroll_force(force)?;
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
    if scroll_force
        .is_some_and(|force| status.scroll.as_ref().and_then(|item| item.force) != Some(force))
    {
        return Err("滚动力度回读不一致".into());
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
    let _ = top_sender().send(TopCommand::Stop);
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

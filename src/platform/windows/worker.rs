//! Hardware I/O is serialized on a separate worker, never on the input/UI thread.
use crate::{
    config,
    devices::{self, DeviceInfo, HidTransport},
    hidpp::{Client, DeviceStatus, ScrollMode},
};
use serde::{Deserialize, Serialize};
use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

pub enum Command {
    Scan,
    Apply {
        dpi: Option<u16>,
        scroll: Option<(u8, u8)>,
    },
    Export {
        path: PathBuf,
    },
    Stop,
}

#[derive(Clone, Debug, Serialize)]
pub struct HardwareView {
    pub summary: String,
    pub details: String,
    pub dpi: Option<u16>,
    pub dpi_choices: Vec<u16>,
    pub scroll_mode: Option<u8>,
    pub threshold: u8,
    pub can_dpi: bool,
    pub can_scroll: bool,
}
impl Default for HardwareView {
    fn default() -> Self {
        Self {
            summary: "设备：尚未探测".into(),
            details: "连接鼠标后点击“查找 / 刷新设备”。".into(),
            dpi: None,
            dpi_choices: vec![],
            scroll_mode: None,
            threshold: 25,
            can_dpi: false,
            can_scroll: false,
        }
    }
}

struct Selected {
    endpoint: DeviceInfo,
    client: Client<HidTransport>,
    status: DeviceStatus,
}
#[derive(Serialize, Deserialize)]
struct SavedHardware {
    version: u8,
    target: String,
    dpi: Option<u16>,
    scroll: Option<(u8, u8)>,
}

pub fn spawn() -> (mpsc::Sender<Command>, mpsc::Receiver<HardwareView>) {
    let (command_tx, command_rx) = mpsc::channel();
    let (event_tx, event_rx) = mpsc::channel();
    std::thread::Builder::new().name("logi-hid".into()).spawn(move||{
        let mut endpoints=Vec::new();let mut selected:Option<Selected>=None;let mut view=HardwareView::default();
        for command in command_rx {
            match command {
                Command::Stop=>break,
                Command::Scan=>{
                    selected=None;
                    match scan() {
                        Ok((devices,found,details))=>{endpoints=devices;selected=found;view=match selected.as_ref(){Some(target)=>status_view(&target.status,&details),None=>HardwareView{summary:"设备：未识别到可通信的 MX Anywhere 3".into(),details,..Default::default()}};}
                        Err(error)=>{view=HardwareView{summary:"设备枚举失败".into(),details:error,..Default::default()};}
                    }
                }
                Command::Apply{dpi,scroll}=>{
                    if let Some(target)=selected.as_mut(){
                        let result=apply(target,dpi,scroll);
                        match result {
                            Ok(message)=>{view=status_view(&target.status,&message);}
                            Err(error)=>{
                                // Re-read when possible; a multi-field write may partially succeed.
                                if let Ok(status)=target.client.probe().and_then(|status|if is_anywhere3(status.name.as_deref().unwrap_or(""),target.endpoint.pid){Ok(status)}else{Err("设备身份已改变".into())}){target.status=status;view=status_view(&target.status,&format!("应用未完全成功：{error}\r\n以上显示为重新读取的实际状态。"));}
                                else {view=HardwareView{summary:"设备通信中断，请重新查找设备".into(),details:format!("应用失败：{error}。部分设置可能已生效。"),..Default::default()};selected=None;}
                            }
                        }
                    }else {view=HardwareView{summary:"没有选定设备".into(),details:"请先连接鼠标并查找设备。".into(),..Default::default()};}
                }
                Command::Export{path}=>{
                    let data=serde_json::json!({"app":"Logi Lite","version":env!("CARGO_PKG_VERSION"),"mapping_scope":"global_all_mice","devices":endpoints,"selected":selected.as_ref().map(|s|&s.status),"view":view,"hardware_response_observed":selected.is_some(),"notes":"A selected entry means read responses were observed, not that every hardware function is tested. Firmware compatibility remains experimental. No serial-number field is collected."});
                    let result=(||{if let Some(parent)=path.parent(){std::fs::create_dir_all(parent).map_err(|e|e.to_string())?;}std::fs::write(&path,serde_json::to_vec_pretty(&data).map_err(|e|e.to_string())?).map_err(|e|e.to_string())})();
                    view.details=match result {Ok(())=>format!("诊断已保存：{}\r\n只包含设备端点、功能响应和错误，不记录鼠标轨迹或键盘内容。",path.display()),Err(error)=>format!("导出诊断失败：{error}")};
                }
            }
            if event_tx.send(view.clone()).is_err(){break;}
        }
    }).expect("cannot create HID worker thread");
    (command_tx, event_rx)
}

fn scan() -> Result<(Vec<DeviceInfo>, Option<Selected>, String), String> {
    let endpoints = devices::discover()?;
    let mut details = devices::format_report(&endpoints);
    let start = Instant::now();
    let mut candidates: Vec<_> = endpoints
        .iter()
        .filter(|d| d.is_hidpp_candidate() && d.output_report_bytes.unwrap_or(0) >= 20)
        .collect();
    candidates.sort_by_key(|d| {
        (
            !matches!(d.pid, 0xb025 | 0xb02d | 0xb037),
            d.output_report_bytes.unwrap_or(0),
        )
    });
    let mut found = Vec::new();
    'endpoints: for endpoint in candidates.into_iter().take(4) {
        for index in endpoint.candidate_indices() {
            if start.elapsed() > Duration::from_secs(14) {
                details.push_str("\r\n探测达到时间上限；可重新刷新设备。");
                break 'endpoints;
            }
            let transport = match HidTransport::open(endpoint) {
                Ok(transport) => transport,
                Err(error) => {
                    details.push_str(&format!("\r\n端点 {:04x} 无法打开：{error}", endpoint.pid));
                    break;
                }
            };
            let mut client = Client::new(transport, index);
            match client.probe() {
                Ok(status) => {
                    let name = status.name.clone().unwrap_or_default();
                    if is_anywhere3(&name, endpoint.pid) {
                        let duplicate = found.iter().any(|old: &Selected| {
                            old.endpoint.path == endpoint.path && old.status.device_index == index
                        });
                        if !duplicate {
                            found.push(Selected {
                                endpoint: endpoint.clone(),
                                client,
                                status,
                            });
                        }
                        // Multiple collections can expose the same receiver slot. One usable
                        // endpoint is enough; don't run more probes against a discovered device.
                        break;
                    } else {
                        details.push_str(&format!(
                            "\r\n槽位 {index}：{}，不是本版目标，未开放写设置。",
                            if name.is_empty() {
                                "未知设备"
                            } else {
                                &name
                            }
                        ));
                    }
                }
                Err(error) => {
                    details.push_str(&format!(
                        "\r\n端点 {:04x} 槽位 {index}：{error}",
                        endpoint.pid
                    ));
                }
            }
        }
        if found.len() == 1 {
            break;
        }
    }
    let selected = found.pop();
    if selected.is_none() {
        details.push_str("\r\n未发现可通信的目标鼠标。可能未连接、处于休眠、端点受限，或 Options+ 正在占用。\r\n蓝牙端点如不暴露 HID++，本版不会猜测 GATT UUID；可尝试平时使用的接收器。\r\n可以先使用托盘与全局快捷键功能。");
    } else {
        details.push_str("\r\n已读取设备响应；硬件功能仍为实验版，请先测试一次设置。\r\n本版只在点击应用时修改 DPI / 滚轮，刷新不会重写硬件设置。");
    }
    Ok((endpoints, selected, details))
}

fn is_anywhere3(name: &str, pid: u16) -> bool {
    let compact = name.to_ascii_lowercase().replace([' ', '-', '_'], "");
    compact.contains("mxanywhere3") || (pid == 0xb025 && name.is_empty())
}

fn status_view(status: &DeviceStatus, detail: &str) -> HardwareView {
    let battery = status
        .battery
        .as_ref()
        .map(|b| {
            b.percent
                .map(|n| format!("{n}%"))
                .or_else(|| {
                    b.level.as_deref().map(|level| {
                        match level {
                            "full" => "满电",
                            "good" => "较高",
                            "low" => "较低",
                            "critical" => "即将耗尽",
                            "empty" => "空电量",
                            _ => "未知",
                        }
                        .into()
                    })
                })
                .unwrap_or_else(|| "百分比不可读".into())
        })
        .unwrap_or_else(|| "暂不可读".into());
    let mode = status.scroll.as_ref().map(|s| match s.mode {
        ScrollMode::Ratchet => 0,
        ScrollMode::Free => 1,
        ScrollMode::SmartShift => 2,
    });
    let name = status
        .name
        .clone()
        .unwrap_or_else(|| "MX Anywhere 3".into());
    let mut details = detail.to_owned();
    if !status.dpi_ranges.is_empty() {
        for range in &status.dpi_ranges {
            details.push_str(&format!(
                "\r\nDPI 支持 {}–{}，步长 {}。",
                range.min, range.max, range.step
            ));
        }
    }
    for warning in &status.warnings {
        details.push_str(&format!("\r\n{warning}"));
    }
    HardwareView {
        summary: format!("设备：{name}   电量：{battery}"),
        details,
        dpi: status.dpi,
        dpi_choices: status.dpi_choices.clone(),
        scroll_mode: mode,
        threshold: status
            .scroll
            .as_ref()
            .map(|s| {
                if (1..=49).contains(&s.threshold) {
                    s.threshold
                } else {
                    25
                }
            })
            .unwrap_or(25),
        can_dpi: status.capabilities.dpi,
        can_scroll: status.capabilities.scroll,
    }
}

fn apply(
    target: &mut Selected,
    dpi: Option<u16>,
    scroll: Option<(u8, u8)>,
) -> Result<String, String> {
    // A receiver slot can be re-paired while its HID endpoint remains open.
    // Rebuild feature indices and verify target identity before any setter.
    let current = target.client.probe()?;
    if !is_anywhere3(current.name.as_deref().unwrap_or(""), target.endpoint.pid) {
        return Err("设备身份已改变，未发送设置，请重新查找设备".into());
    }
    target.status = current;
    let mut changes = Vec::new();
    if let Some(dpi) = dpi
        && target.status.dpi != Some(dpi)
    {
        target.client.set_dpi(dpi)?;
        changes.push(format!("DPI {dpi}"));
    }
    if let Some((mode, threshold)) = scroll {
        let mode = match mode {
            0 => ScrollMode::Ratchet,
            1 => ScrollMode::Free,
            2 => ScrollMode::SmartShift,
            _ => return Err("无效滚轮模式".into()),
        };
        let same = target
            .status
            .scroll
            .as_ref()
            .is_some_and(|old| old.mode == mode && old.threshold == threshold);
        if !same {
            target.client.set_scroll(mode, threshold)?;
            changes.push("滚轮设置".into());
        }
    }
    target.status = target.client.probe()?;
    if let Some(dpi) = dpi
        && target.status.dpi != Some(dpi)
    {
        return Err("DPI 回读与请求不一致，未保存为成功配置".into());
    }
    if let Some((mode, threshold)) = scroll {
        let expected = match mode {
            0 => ScrollMode::Ratchet,
            1 => ScrollMode::Free,
            _ => ScrollMode::SmartShift,
        };
        if target.status.scroll.as_ref().is_none_or(|s| {
            s.mode != expected || (expected == ScrollMode::SmartShift && s.threshold != threshold)
        }) {
            return Err("滚轮回读与请求不一致，未保存为成功配置".into());
        }
    }
    let preference = SavedHardware {
        version: 1,
        target: target
            .status
            .name
            .clone()
            .unwrap_or_else(|| "MX Anywhere 3".into()),
        dpi,
        scroll,
    };
    let path = config::config_path()?.with_file_name("hardware-settings.json");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&preference).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("鼠标设置已生效，但保存记录失败：{e}"))?;
    Ok(format!(
        "已应用并回读{}。\r\n设置记录已保存在 {}。\r\n本版不在重连时自动写入；若鼠标重连后恢复默认，请刷新后重新应用。",
        if changes.is_empty() {
            "（设置未改变）".into()
        } else {
            changes.join("、")
        },
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_expected_models_are_selected() {
        assert!(is_anywhere3("MX Anywhere 3", 0xc52b));
        assert!(is_anywhere3("MX Anywhere 3S", 0xc548));
        assert!(is_anywhere3("", 0xb025));
        assert!(!is_anywhere3("MX Master 3", 0xc52b));
        assert!(!is_anywhere3("", 0xc52b));
        assert!(!is_anywhere3("Keyboard", 0xb025));
    }
}

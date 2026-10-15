use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub enabled: bool,
    pub back: String,
    pub forward: String,
    pub middle: String,
    // Accept and discard the retired field when reading experiment-era files.
    #[cfg(target_os = "macos")]
    #[serde(
        default,
        rename = "gestures",
        skip_serializing,
        deserialize_with = "discard_retired_gestures"
    )]
    pub retired_gestures: (),
}

#[cfg(target_os = "macos")]
fn discard_retired_gestures<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<(), D::Error> {
    serde::de::IgnoredAny::deserialize(decoder)?;
    Ok(())
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            enabled: false,
            back: "默认".into(),
            forward: "默认".into(),
            middle: "默认".into(),
            #[cfg(target_os = "macos")]
            retired_gestures: (),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Default,
    Disabled,
    Shortcut(Chord),
    #[cfg(target_os = "macos")]
    System,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Chord {
    pub modifiers: Vec<u16>,
    pub key: u16,
}

pub(crate) fn parse_action(text: &str) -> Result<Action, String> {
    let text = text.trim();
    if text == "默认" {
        return Ok(Action::Default);
    }
    if text == "禁用" {
        return Ok(Action::Disabled);
    }
    #[cfg(target_os = "macos")]
    if matches!(
        text,
        "打开 App"
            | "调度中心"
            | "水平滚动"
            | "系统设置"
            | "计算器"
            | "音量增加"
            | "音量减小"
            | "静音切换"
            | "播放或暂停"
            | "下一首"
            | "上一首"
            | "亮度增加"
            | "亮度减小"
    ) || text.starts_with("打开应用:")
    {
        if text.starts_with("打开应用:") {
            let path = text.strip_prefix("打开应用:").unwrap();
            if !path.starts_with('/') || !path.ends_with(".app") {
                return Err("应用必须是绝对路径的 .app".into());
            }
        }
        return Ok(Action::System);
    }
    let parts: Vec<_> = text.split('+').map(str::trim).collect();
    let mut modifiers = Vec::new();
    for part in &parts[..parts.len().saturating_sub(1)] {
        let key = match part.to_ascii_uppercase().as_str() {
            "CTRL" => 0x11,
            #[cfg(target_os = "macos")]
            "CONTROL" => 0x11,
            "ALT" => 0x12,
            #[cfg(target_os = "macos")]
            "OPTION" => 0x12,
            "SHIFT" => 0x10,
            "WIN" => 0x5B,
            #[cfg(target_os = "macos")]
            "CMD" | "COMMAND" => 0x5B,
            _ => return Err(format!("未知修饰键：{part}；支持 Ctrl、Alt、Shift、Win")),
        };
        if modifiers.contains(&key) {
            return Err(format!("修饰键重复：{part}"));
        }
        modifiers.push(key);
    }
    let last = parts.last().copied().unwrap_or("").to_ascii_uppercase();
    let key = match last.as_str() {
        "ENTER" => 0x0D,
        "SPACE" => 0x20,
        "TAB" => 0x09,
        "ESC" => 0x1B,
        "LEFT" => 0x25,
        "UP" => 0x26,
        "RIGHT" => 0x27,
        "DOWN" => 0x28,
        "HOME" => 0x24,
        "END" => 0x23,
        "DELETE" => 0x2E,
        "PAGEUP" => 0x21,
        "PAGEDOWN" => 0x22,
        value if value.len() == 1 && value.as_bytes()[0].is_ascii_alphanumeric() => {
            value.as_bytes()[0] as u16
        }
        value if value.starts_with('F') => {
            let number = value[1..]
                .parse::<u16>()
                .map_err(|_| "功能键必须是 F1～F24".to_string())?;
            if !(1..=24).contains(&number) {
                return Err("功能键必须是 F1～F24".into());
            }
            #[cfg(target_os = "macos")]
            if number > 20 {
                return Err("macOS 支持 F1～F20".into());
            }
            0x70 + number - 1
        }
        _ => {
            return Err(format!(
                "无效快捷键：{text}；例如 Ctrl+C、Alt+Left、Win+Tab 或 F5"
            ));
        }
    };
    Ok(Action::Shortcut(Chord { modifiers, key }))
}

pub fn validate(config: &Config) -> Result<(), String> {
    if config.version != 1 {
        return Err(format!("不支持配置版本 {}", config.version));
    }
    for (name, value) in [
        ("后退侧键", &config.back),
        ("前进侧键", &config.forward),
        ("中键", &config.middle),
    ] {
        parse_action(value).map_err(|error| format!("{name}：{error}"))?;
    }
    Ok(())
}

pub fn config_path() -> Result<PathBuf, String> {
    crate::platform::storage::config_path()
}

fn load_path(path: &Path) -> Result<Config, String> {
    let bytes = match std::fs::read(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(error) => return Err(format!("无法读取配置 {}：{error}", path.display())),
    };
    #[allow(unused_mut)]
    let mut config: Config = serde_json::from_slice(&bytes)
        .map_err(|error| format!("配置格式错误，原文件已保留 {}：{error}", path.display()))?;
    #[cfg(target_os = "macos")]
    for value in [&mut config.back, &mut config.forward, &mut config.middle] {
        if value.trim() == "手势按钮" {
            *value = "默认".into();
            config.enabled = false;
        }
    }
    validate(&config)?;
    Ok(config)
}

pub fn load() -> Result<Config, String> {
    load_path(&config_path()?)
}

pub fn save(config: &Config) -> Result<(), String> {
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    validate(config)?;
    let path = config_path()?;
    let parent = path.parent().ok_or("配置目录无效")?;
    std::fs::create_dir_all(parent).map_err(|error| format!("创建配置目录失败：{error}"))?;
    let temporary = parent.join(format!(
        "config.{}.{}.tmp",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let bytes = serde_json::to_vec_pretty(config).map_err(|error| error.to_string())?;
    // Only delete a temporary file that this save operation actually created.
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("创建配置临时文件失败：{error}"))?;
    let result = (|| {
        file.write_all(&bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        crate::platform::storage::replace_file(&temporary, &path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|error: String| format!("保存配置失败：{error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parser_accepts_supported_chords() {
        for value in [
            "默认",
            "禁用",
            "ctrl + shift + C",
            "Alt+Left",
            "Win+Tab",
            "F20",
            "Space",
            "Ctrl+0",
            "Ctrl+PageDown",
        ] {
            assert!(parse_action(value).is_ok(), "{value}");
        }
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn mac_modifiers_match_native_input() {
        for value in [
            "Cmd+C",
            "Command+Option+Left",
            "Control+Shift+F20",
            "Win+Tab",
            "打开 App",
            "调度中心",
            "水平滚动",
            "打开应用:/Applications/Test.app",
            "系统设置",
            "计算器",
            "音量增加",
            "音量减小",
            "静音切换",
            "播放或暂停",
            "下一首",
            "上一首",
            "亮度增加",
            "亮度减小",
        ] {
            assert!(parse_action(value).is_ok(), "{value}");
        }
        for value in [
            "Cmd+Win+C",
            "Alt+Option+Left",
            "F21",
            "F24",
            "打开应用:Test.app",
            "打开应用:/tmp/test.sh",
        ] {
            assert!(parse_action(value).is_err(), "{value}");
        }
    }
    #[test]
    fn parser_rejects_ambiguous_or_incomplete_chords() {
        for value in [
            "",
            "Ctrl",
            "Ctrl+",
            "Ctrl+Ctrl+C",
            "Ctrl+C+V",
            "F0",
            "F25",
            "Ctrl++",
            "Super+C",
            "汉",
        ] {
            assert!(parse_action(value).is_err(), "{value}");
        }
    }
    #[test]
    fn defaults_are_opt_in_and_unknown_versions_are_rejected() {
        let mut config = Config::default();
        assert!(!config.enabled);
        assert_eq!(parse_action(&config.back).unwrap(), Action::Default);
        config.version = 2;
        assert!(validate(&config).is_err());
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn retired_gestures_load_safely_without_rewriting_the_original() {
        let path = std::env::temp_dir().join(format!(
            "logilite-retired-gestures-{}.json",
            std::process::id()
        ));
        let old = r#"{"version":1,"enabled":true,"back":"手势按钮","forward":"打开 App","middle":"Cmd+C","gestures":{"click":"禁用","distance":40}}"#;
        std::fs::write(&path, old).unwrap();
        let config = load_path(&path).unwrap();
        assert_eq!(config.back, "默认");
        assert_eq!(config.forward, "打开 App");
        assert_eq!(config.middle, "Cmd+C");
        assert!(!config.enabled);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), old);
        assert!(
            !serde_json::to_value(&config)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("gestures")
        );
        assert!(parse_action("手势按钮").is_err());
        let regular = old.replace("手势按钮", "调度中心");
        std::fs::write(&path, regular).unwrap();
        let config = load_path(&path).unwrap();
        assert!(config.enabled);
        assert_eq!(config.back, "调度中心");
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn malformed_file_is_preserved() {
        let path =
            std::env::temp_dir().join(format!("logilite-bad-config-{}.json", std::process::id()));
        std::fs::write(&path, b"{bad").unwrap();
        assert!(load_path(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{bad");
        std::fs::remove_file(path).unwrap();
    }
}

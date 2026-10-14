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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            enabled: false,
            back: "默认".into(),
            forward: "默认".into(),
            middle: "默认".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Action {
    Default,
    Disabled,
    Shortcut(Chord),
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
    let parts: Vec<_> = text.split('+').map(str::trim).collect();
    let mut modifiers = Vec::new();
    for part in &parts[..parts.len().saturating_sub(1)] {
        let key = match part.to_ascii_uppercase().as_str() {
            "CTRL" => 0x11,
            "ALT" => 0x12,
            "SHIFT" => 0x10,
            "WIN" => 0x5B,
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
    let root = std::env::var_os("LOCALAPPDATA")
        .filter(|value| !value.is_empty())
        .ok_or("无法获取 LOCALAPPDATA 配置目录")?;
    Ok(PathBuf::from(root).join("LogiLite").join("config.json"))
}

fn load_path(path: &Path) -> Result<Config, String> {
    let bytes = match std::fs::read(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Config::default()),
        Err(error) => return Err(format!("无法读取配置 {}：{error}", path.display())),
    };
    let config: Config = serde_json::from_slice(&bytes)
        .map_err(|error| format!("配置格式错误，原文件已保留 {}：{error}", path.display()))?;
    validate(&config)?;
    Ok(config)
}

pub fn load() -> Result<Config, String> {
    load_path(&config_path()?)
}

pub fn save(config: &Config) -> Result<(), String> {
    use std::io::Write;
    use std::os::windows::ffi::OsStrExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
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
        let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // Atomic replacement on the same volume; preserve the original on failure.
        if unsafe {
            MoveFileExW(
                from.as_ptr(),
                to.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
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
            "F24",
            "Space",
            "Ctrl+0",
            "Ctrl+PageDown",
        ] {
            assert!(parse_action(value).is_ok(), "{value}");
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

//! macOS configuration adapter; native mouse integration is a separate next step.
use std::path::{Path, PathBuf};

pub fn config_path() -> Result<PathBuf, String> {
    let root = std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .ok_or("无法获取 macOS 用户配置目录")?;
    Ok(PathBuf::from(root)
        .join("Library")
        .join("Application Support")
        .join("LogiLite")
        .join("config.json"))
}

pub fn replace_file(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::rename(from, to).map_err(|error| error.to_string())
}

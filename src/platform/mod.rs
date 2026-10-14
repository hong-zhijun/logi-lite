//! Native UI, input, device transport and filesystem adapters.
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub(crate) use macos::storage;
#[cfg(windows)]
pub(crate) use windows::storage;

#[cfg(not(any(windows, target_os = "macos")))]
pub(crate) mod storage {
    use std::path::{Path, PathBuf};
    pub fn config_path() -> Result<PathBuf, String> {
        Err("This platform has no configuration adapter".into())
    }
    pub fn replace_file(_: &Path, _: &Path) -> Result<(), String> {
        Err("This platform has no configuration adapter".into())
    }
}

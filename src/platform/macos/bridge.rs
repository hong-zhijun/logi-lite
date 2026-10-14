//! Narrow C interface to the independently implemented AppKit / IOKit adapters.
//! Returned JSON owns its allocation; Swift releases it using free_string.
use crate::{config, hidpp};
use serde::Deserialize;
use serde_json::{Value, json};
use std::ffi::{CStr, CString, c_char, c_void};
use std::sync::atomic::{AtomicU64, Ordering};

pub type Exchange = unsafe extern "C" fn(*mut c_void, *const u8, usize, *mut u8, usize) -> i32;

struct NativeTransport {
    context: *mut c_void,
    exchange: Exchange,
}

impl hidpp::Transport for NativeTransport {
    fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, String> {
        let mut reply = [0u8; 64];
        // The synchronous Swift callback borrows this context only for this call.
        let count = unsafe {
            (self.exchange)(
                self.context,
                request.as_ptr(),
                request.len(),
                reply.as_mut_ptr(),
                reply.len(),
            )
        };
        if count <= 0 || count as usize > reply.len() {
            return Err(format!(
                "macOS HID transport failed ({count}); 请检查权限或重新刷新设备"
            ));
        }
        Ok(reply[..count as usize].to_vec())
    }
}

fn response(result: Result<Value, String>) -> *mut c_char {
    let value = match result {
        Ok(value) => value,
        Err(error) => json!({"ok":false,"error":error}),
    };
    // JSON serialization escapes interior NUL characters.
    CString::new(value.to_string())
        .expect("JSON contains no interior NUL")
        .into_raw()
}

unsafe fn input_json<T: for<'a> Deserialize<'a>>(input: *const c_char) -> Result<T, String> {
    if input.is_null() {
        return Err("Missing JSON input".into());
    }
    let bytes = unsafe { CStr::from_ptr(input) }.to_bytes();
    if bytes.len() > 64 * 1024 {
        return Err("JSON input exceeds limit".into());
    }
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

#[unsafe(no_mangle)]
pub extern "C" fn logilite_load_config() -> *mut c_char {
    response(config::load().map(|config| json!({"ok":true,"config":config})))
}

/// # Safety
/// `input` must be a live NUL-terminated UTF-8 string for the duration of the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn logilite_save_config(input: *const c_char) -> *mut c_char {
    response((|| {
        let config: config::Config = unsafe { input_json(input) }?;
        config::save(&config)?;
        Ok(json!({"ok":true,"config":config}))
    })())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HardwareCommand {
    action: String,
    #[serde(default = "direct_index")]
    device_index: u8,
    dpi: Option<u16>,
    scroll: Option<hidpp::ScrollMode>,
    sensitivity: Option<u8>,
}
fn direct_index() -> u8 {
    255
}

fn is_target(status: &hidpp::DeviceStatus) -> bool {
    status.name.as_deref().is_some_and(|name| {
        name.to_ascii_lowercase()
            .replace([' ', '-', '_'], "")
            .starts_with("mxanywhere3")
    })
}

/// # Safety
/// `input` is a live NUL-terminated UTF-8 string. `exchange` must synchronously
/// read its request and write at most capacity bytes to its response. `context`
/// remains live throughout this call; it is never stored or shared by Rust.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn logilite_hardware(
    context: *mut c_void,
    exchange: Option<Exchange>,
    input: *const c_char,
) -> *mut c_char {
    response((|| {
        let command: HardwareCommand = unsafe { input_json(input) }?;
        if command.action != "probe" && command.action != "apply" {
            return Err("Unknown hardware command".into());
        }
        if command.device_index != 255 && !(1..=6).contains(&command.device_index) {
            return Err("Invalid HID++ device index".into());
        }
        // Validate every requested value before the first write.
        if command.action == "probe"
            && (command.dpi.is_some() || command.scroll.is_some() || command.sensitivity.is_some())
        {
            return Err("A probe cannot include hardware writes".into());
        }
        if command.sensitivity.is_some() && command.scroll != Some(hidpp::ScrollMode::SmartShift) {
            return Err("Sensitivity requires SmartShift mode".into());
        }
        if let Some(sensitivity) = command.sensitivity {
            hidpp::smartshift_threshold(sensitivity)?;
        }
        static SESSION_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
            ^ SESSION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut client = hidpp::Client::new(
            NativeTransport {
                context,
                exchange: exchange.ok_or("Missing HID exchange callback")?,
            },
            command.device_index,
        )
        .with_session_nonce(nonce);
        let status = client.probe()?;
        if command.action == "probe" {
            return Ok(json!({"ok":true,"status":status,"target_verified":is_target(&status)}));
        }
        if !is_target(&status) {
            return Err("设备未确认是 MX Anywhere 3 系列，拒绝写设置".into());
        }
        if let Some(dpi) = command.dpi
            && (!status.capabilities.dpi
                || (!status.dpi_choices.contains(&dpi)
                    && !status.dpi_ranges.iter().any(|range| {
                        dpi >= range.min
                            && dpi <= range.max
                            && range.step > 0
                            && (dpi - range.min).is_multiple_of(range.step)
                    })))
        {
            return Err("DPI 不在设备支持的范围内".into());
        }
        if command.scroll.is_some() && !status.capabilities.scroll {
            return Err("设备未确认支持滚轮设置".into());
        }
        let threshold = match command.sensitivity {
            Some(sensitivity) => hidpp::smartshift_threshold(sensitivity)?,
            None => status
                .scroll
                .as_ref()
                .filter(|s| s.mode == hidpp::ScrollMode::SmartShift)
                .map(|s| s.threshold)
                .unwrap_or(hidpp::smartshift_threshold(50)?),
        };
        let applied = (|| {
            if let Some(dpi) = command.dpi {
                client.set_dpi(dpi)?;
            }
            if let Some(mode) = command.scroll {
                client.set_scroll(mode, threshold)?;
            }
            Ok::<(), String>(())
        })();
        // Always try a fresh read, including after a partial application failure.
        let refreshed = client.probe();
        match (applied, refreshed) {
            (Ok(()), Ok(status)) if is_target(&status) => {
                if command.dpi.is_some_and(|dpi| status.dpi != Some(dpi))
                    || command.scroll.is_some_and(|mode| {
                        status.scroll.as_ref().is_none_or(|s| {
                            s.mode != mode
                                || (mode == hidpp::ScrollMode::SmartShift
                                    && s.threshold != threshold)
                        })
                    })
                {
                    Ok(
                        json!({"ok":false,"status":status,"error":"写入后的回读值不一致，请以实际状态为准"}),
                    )
                } else {
                    Ok(json!({"ok":true,"status":status,"target_verified":true}))
                }
            }
            (Err(error), Ok(status)) => Ok(
                json!({"ok":false,"status":status,"error":format!("部分设置可能已生效：{error}")}),
            ),
            (applied, _) => Err(format!(
                "无法回读确认，部分设置可能已生效：{}",
                applied.err().unwrap_or_else(|| "设备通信中断".into())
            )),
        }
    })())
}

/// # Safety
/// Pass only a non-null pointer returned by this bridge, exactly once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn logilite_free_string(pointer: *mut c_char) {
    if !pointer.is_null() {
        drop(unsafe { CString::from_raw(pointer) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe extern "C" fn never_exchange(
        _: *mut c_void,
        _: *const u8,
        _: usize,
        _: *mut u8,
        _: usize,
    ) -> i32 {
        panic!("invalid commands must never reach hardware");
    }
    #[test]
    fn invalid_commands_do_not_touch_hardware() {
        for text in [
            r#"{"action":"probe","dpi":400}"#,
            r#"{"action":"apply","threshold":20}"#,
            r#"{"action":"apply","scroll":"smart_shift","threshold":0}"#,
            r#"{"action":"apply","scroll":"smart_shift","sensitivity":101}"#,
            r#"{"action":"apply","sensitivity":50}"#,
            r#"{"action":"apply","scroll":"ratchet","sensitivity":50}"#,
            r#"{"action":"apply","device_index":0}"#,
        ] {
            let input = CString::new(text).unwrap();
            let pointer = unsafe {
                logilite_hardware(std::ptr::null_mut(), Some(never_exchange), input.as_ptr())
            };
            let value: Value =
                serde_json::from_slice(unsafe { CStr::from_ptr(pointer) }.to_bytes()).unwrap();
            assert_eq!(value["ok"], false);
            unsafe { logilite_free_string(pointer) };
        }
    }
}

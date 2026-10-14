//! Windows HID discovery and explicit, bounded HID++ transport.
//! Discovery opens metadata-only handles and never sends HID reports.
use serde::Serialize;
use std::{
    mem::size_of,
    ptr::{null, null_mut},
    time::{Duration, Instant},
};
use windows_sys::{
    Win32::{
        Devices::{DeviceAndDriverInstallation::*, HumanInterfaceDevice::*},
        Foundation::*,
        Storage::FileSystem::*,
        System::{IO::*, Threading::*},
    },
    core::GUID,
};

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub product: Option<String>,
    pub manufacturer: Option<String>,
    pub vid: u16,
    pub pid: u16,
    pub path: String,
    pub usage_page: Option<u16>,
    pub usage: Option<u16>,
    pub input_report_bytes: Option<u16>,
    pub output_report_bytes: Option<u16>,
    pub feature_report_bytes: Option<u16>,
}

impl DeviceInfo {
    pub fn is_hidpp_candidate(&self) -> bool {
        self.vid == 0x046d
            && self.usage_page.is_some_and(|page| page >= 0xff00)
            && matches!(self.usage, Some(1 | 2))
            && self.input_report_bytes.is_some_and(|length| length >= 7)
            && self.output_report_bytes.is_some_and(|length| length >= 7)
    }

    /// Known Unifying / Bolt receivers. Other endpoints should try direct index 255.
    /// A receiver interface is not evidence that a paired mouse is connected.
    pub fn is_receiver(&self) -> bool {
        matches!(self.pid, 0xc52b | 0xc532 | 0xc548)
    }

    pub fn candidate_indices(&self) -> Vec<u8> {
        if self.is_receiver() {
            (1..=6).collect()
        } else {
            vec![255]
        }
    }
}

struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
struct DeviceSet(HDEVINFO);
impl Drop for DeviceSet {
    fn drop(&mut self) {
        unsafe {
            SetupDiDestroyDeviceInfoList(self.0);
        }
    }
}
struct Preparsed(PHIDP_PREPARSED_DATA);
impl Drop for Preparsed {
    fn drop(&mut self) {
        unsafe {
            HidD_FreePreparsedData(self.0);
        }
    }
}
fn win_error(context: &str, code: u32) -> String {
    format!(
        "{context}: {} (Windows {code})",
        std::io::Error::from_raw_os_error(code as i32)
    )
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

fn hid_string(handle: HANDLE, manufacturer: bool) -> Option<String> {
    // HID string buffers must not exceed 4093 bytes. 256 UTF-16 characters is sufficient.
    let mut buffer = [0u16; 256];
    let ok = unsafe {
        if manufacturer {
            HidD_GetManufacturerString(
                handle,
                buffer.as_mut_ptr().cast(),
                size_of::<[u16; 256]>() as u32,
            )
        } else {
            HidD_GetProductString(
                handle,
                buffer.as_mut_ptr().cast(),
                size_of::<[u16; 256]>() as u32,
            )
        }
    };
    if !ok {
        let _ = unsafe { GetLastError() };
        return None;
    }
    let end = buffer.iter().position(|&item| item == 0)?;
    let text = String::from_utf16_lossy(&buffer[..end]).trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// Enumerate present Logitech HID collections. This does not imply a mouse is connected.
pub fn discover() -> Result<Vec<DeviceInfo>, String> {
    unsafe {
        let mut guid = GUID::default();
        HidD_GetHidGuid(&mut guid);
        let raw_set = SetupDiGetClassDevsW(
            &guid,
            null(),
            null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );
        if raw_set == -1 {
            return Err(win_error("HID enumeration unavailable", GetLastError()));
        }
        let set = DeviceSet(raw_set);
        let mut devices = Vec::new();
        for index in 0.. {
            let mut interface = SP_DEVICE_INTERFACE_DATA {
                cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };
            if SetupDiEnumDeviceInterfaces(set.0, null(), &guid, index, &mut interface) == 0 {
                let error = GetLastError();
                if error == ERROR_NO_MORE_ITEMS {
                    break;
                }
                return Err(win_error("HID interface enumeration failed", error));
            }
            let mut required = 0;
            let size_ok = SetupDiGetDeviceInterfaceDetailW(
                set.0,
                &interface,
                null_mut(),
                0,
                &mut required,
                null_mut(),
            );
            let size_error = if size_ok == 0 { GetLastError() } else { 0 };
            if size_ok == 0 && size_error != ERROR_INSUFFICIENT_BUFFER {
                return Err(win_error("HID interface detail size failed", size_error));
            }
            if !(6..=1024 * 1024).contains(&required) {
                return Err("Invalid HID interface path size".into());
            }
            // usize allocation supplies alignment on both 32-bit and 64-bit targets.
            let words = (required as usize).div_ceil(size_of::<usize>());
            let mut storage = vec![0usize; words];
            let detail = storage
                .as_mut_ptr()
                .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
            (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
            if SetupDiGetDeviceInterfaceDetailW(
                set.0,
                &interface,
                detail,
                required,
                null_mut(),
                null_mut(),
            ) == 0
            {
                return Err(win_error("HID interface detail failed", GetLastError()));
            }
            let path_offset = std::mem::offset_of!(SP_DEVICE_INTERFACE_DETAIL_DATA_W, DevicePath);
            let path_units = std::slice::from_raw_parts(
                storage.as_ptr().cast::<u8>().add(path_offset).cast::<u16>(),
                (required as usize - path_offset) / 2,
            );
            let end = path_units
                .iter()
                .position(|&value| value == 0)
                .ok_or("Unterminated HID device path")?;
            let path = String::from_utf16(path_units.get(..end).ok_or("Invalid HID device path")?)
                .map_err(|_| "Invalid HID path UTF-16")?;
            let path_wide = wide(&path);
            let raw = CreateFileW(
                path_wide.as_ptr(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null(),
                OPEN_EXISTING,
                0,
                null_mut(),
            );
            if raw == INVALID_HANDLE_VALUE {
                let _ = GetLastError();
                continue;
            }
            let handle = OwnedHandle(raw);
            let mut attributes = HIDD_ATTRIBUTES {
                Size: size_of::<HIDD_ATTRIBUTES>() as u32,
                ..Default::default()
            };
            if !HidD_GetAttributes(handle.0, &mut attributes) {
                let _ = GetLastError();
                continue;
            }
            if attributes.VendorID != 0x046d {
                continue;
            }
            let mut caps = None;
            let mut preparsed = 0;
            if HidD_GetPreparsedData(handle.0, &mut preparsed) {
                let preparsed = Preparsed(preparsed);
                let mut data = HIDP_CAPS::default();
                if HidP_GetCaps(preparsed.0, &mut data) == HIDP_STATUS_SUCCESS {
                    caps = Some(data);
                }
            } else {
                let _ = GetLastError();
            }
            devices.push(DeviceInfo {
                product: hid_string(handle.0, false),
                manufacturer: hid_string(handle.0, true),
                vid: attributes.VendorID,
                pid: attributes.ProductID,
                path,
                usage_page: caps.as_ref().map(|c| c.UsagePage),
                usage: caps.as_ref().map(|c| c.Usage),
                input_report_bytes: caps.as_ref().map(|c| c.InputReportByteLength),
                output_report_bytes: caps.as_ref().map(|c| c.OutputReportByteLength),
                feature_report_bytes: caps.as_ref().map(|c| c.FeatureReportByteLength),
            });
        }
        Ok(devices)
    }
}

pub fn format_report(devices: &[DeviceInfo]) -> String {
    let mut report = format!(
        "Logitech HID collections: {}\r\nHID++ candidate collections: {}\r\n\r\n",
        devices.len(),
        devices.iter().filter(|d| d.is_hidpp_candidate()).count()
    );
    for (index, device) in devices.iter().enumerate() {
        report.push_str(&format!("{}. {} (VID {:04X}, PID {:04X})\r\n   Usage page {:?}, usage {:?}; report bytes IN {:?} / OUT {:?} / FEATURE {:?}\r\n", index + 1, device.product.as_deref().unwrap_or("Logitech device"), device.vid, device.pid, device.usage_page, device.usage, device.input_report_bytes, device.output_report_bytes, device.feature_report_bytes));
    }
    report.push_str("\r\nThis scan reads HID metadata only. A receiver or HID collection does not confirm a connected mouse. Inaccessible collections may be omitted. Battery, DPI and model capabilities require an explicit HID++ probe.\r\n");
    report
}

pub fn diagnostic_json() -> Result<String, String> {
    serde_json::to_string_pretty(&discover()?).map_err(|error| error.to_string())
}

pub struct HidTransport {
    handle: OwnedHandle,
    input_size: usize,
    output_size: usize,
}

impl HidTransport {
    /// Opens only a previously discovered Logitech vendor collection. No reports sent.
    pub fn open(device: &DeviceInfo) -> Result<Self, String> {
        if !device.is_hidpp_candidate() {
            return Err("Not a Logitech HID++ vendor collection".into());
        }
        let path = wide(&device.path);
        let raw = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED,
                null_mut(),
            )
        };
        if raw == INVALID_HANDLE_VALUE {
            return Err(win_error("Cannot open HID++ endpoint", unsafe {
                GetLastError()
            }));
        }
        let handle = OwnedHandle(raw);
        // Validate the actual handle again; do not trust caller-supplied DeviceInfo.
        let mut attributes = HIDD_ATTRIBUTES {
            Size: size_of::<HIDD_ATTRIBUTES>() as u32,
            ..Default::default()
        };
        if !unsafe { HidD_GetAttributes(handle.0, &mut attributes) } {
            return Err(win_error("Cannot verify HID++ endpoint", unsafe {
                GetLastError()
            }));
        }
        if attributes.VendorID != 0x046d {
            return Err("Endpoint is not Logitech".into());
        }
        let mut pointer = 0;
        if !unsafe { HidD_GetPreparsedData(handle.0, &mut pointer) } {
            return Err(win_error("Cannot read HID report descriptor", unsafe {
                GetLastError()
            }));
        }
        let preparsed = Preparsed(pointer);
        let mut caps = HIDP_CAPS::default();
        if unsafe { HidP_GetCaps(preparsed.0, &mut caps) } != HIDP_STATUS_SUCCESS {
            return Err("Cannot decode HID report descriptor".into());
        }
        if caps.UsagePage < 0xff00
            || !matches!(caps.Usage, 1 | 2)
            || caps.InputReportByteLength < 7
            || caps.OutputReportByteLength < 7
        {
            return Err("Endpoint is not a supported HID++ vendor collection".into());
        }
        Ok(Self {
            handle,
            input_size: caps.InputReportByteLength as usize,
            output_size: caps.OutputReportByteLength as usize,
        })
    }

    pub fn transaction(&mut self, request: &[u8], timeout: Duration) -> Result<Vec<u8>, String> {
        if !matches!(
            (request.first(), request.len()),
            (Some(0x10), 7) | (Some(0x11), 20)
        ) || request.len() > self.output_size
        {
            return Err("HID++ request does not fit endpoint output report".into());
        }
        let deadline = Instant::now() + timeout.min(Duration::from_secs(5));
        let mut output = vec![0u8; self.output_size];
        output[..request.len()].copy_from_slice(request);
        // The read is posted first, so a fast reply cannot be missed while writing.
        let mut read = PendingIo::read(self.handle.0, self.input_size)?;
        let mut write = PendingIo::write(self.handle.0, output)?;
        let written = write.finish(deadline)?;
        if written != self.output_size {
            return Err("Incomplete HID++ output report".into());
        }
        loop {
            let count = read.finish(deadline)?;
            if response_matches(request, &read.buffer[..count]) {
                return Ok(read.buffer[..count].to_vec());
            }
            if Instant::now() >= deadline {
                return Err("HID++ response timeout".into());
            }
            read = PendingIo::read(self.handle.0, self.input_size)?;
        }
    }
}

impl crate::hidpp::Transport for HidTransport {
    fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, String> {
        self.transaction(request, Duration::from_millis(700))
    }
}

fn response_matches(request: &[u8], response: &[u8]) -> bool {
    if request.len() < 4
        || response.len() < 4
        || !matches!(response[0], 0x10 | 0x11)
        || !(response[1] == request[1] || (request[1] == 255 && response[1] == 0))
    {
        return false;
    }
    (response[2] == request[2] && response[3] == request[3])
        || (response.len() >= 6
            && matches!(response[2], 0x8f | 0xff)
            && response[3] == request[2]
            && response[4] == request[3])
}

/// Owns the buffer, event and stable OVERLAPPED until completion or cancellation.
struct PendingIo {
    file: HANDLE,
    event: OwnedHandle,
    overlapped: Box<OVERLAPPED>,
    buffer: Vec<u8>,
    pending: bool,
}
impl PendingIo {
    fn read(file: HANDLE, size: usize) -> Result<Self, String> {
        Self::start(file, vec![0; size], false)
    }
    fn write(file: HANDLE, buffer: Vec<u8>) -> Result<Self, String> {
        Self::start(file, buffer, true)
    }
    fn start(file: HANDLE, buffer: Vec<u8>, writing: bool) -> Result<Self, String> {
        let event = unsafe { CreateEventW(null(), 1, 0, null()) };
        if event.is_null() {
            return Err(win_error("Cannot create HID I/O event", unsafe {
                GetLastError()
            }));
        }
        let mut operation = Self {
            file,
            event: OwnedHandle(event),
            overlapped: Box::new(OVERLAPPED {
                hEvent: event,
                ..Default::default()
            }),
            buffer,
            pending: false,
        };
        let ok = unsafe {
            if writing {
                WriteFile(
                    file,
                    operation.buffer.as_ptr(),
                    operation.buffer.len() as u32,
                    null_mut(),
                    operation.overlapped.as_mut(),
                )
            } else {
                ReadFile(
                    file,
                    operation.buffer.as_mut_ptr(),
                    operation.buffer.len() as u32,
                    null_mut(),
                    operation.overlapped.as_mut(),
                )
            }
        };
        if ok == 0 {
            let error = unsafe { GetLastError() };
            if error != ERROR_IO_PENDING {
                return Err(win_error("HID report I/O failed", error));
            }
        }
        operation.pending = true;
        Ok(operation)
    }
    fn finish(&mut self, deadline: Instant) -> Result<usize, String> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let millis = remaining.as_millis().min(u32::MAX as u128 - 1) as u32;
        match unsafe { WaitForSingleObject(self.event.0, millis) } {
            WAIT_OBJECT_0 => {}
            WAIT_TIMEOUT => return Err("HID++ response timeout".into()),
            _ => return Err(win_error("HID I/O wait failed", unsafe { GetLastError() })),
        }
        let mut count = 0;
        let ok = unsafe { GetOverlappedResult(self.file, self.overlapped.as_ref(), &mut count, 0) };
        let error = if ok == 0 {
            unsafe { GetLastError() }
        } else {
            0
        };
        self.pending = error == ERROR_IO_INCOMPLETE;
        if ok == 0 {
            return Err(win_error("HID report completion failed", error));
        }
        if count as usize > self.buffer.len() {
            return Err("Invalid HID report completion size".into());
        }
        Ok(count as usize)
    }
}
impl Drop for PendingIo {
    fn drop(&mut self) {
        if self.pending {
            unsafe {
                CancelIoEx(self.file, self.overlapped.as_ref());
                let mut transferred = 0;
                // Drain cancellation before freeing the kernel-referenced buffer / OVERLAPPED.
                GetOverlappedResult(self.file, self.overlapped.as_ref(), &mut transferred, 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_mouse_collections_are_never_protocol_candidates() {
        let mut device = DeviceInfo {
            product: None,
            manufacturer: None,
            vid: 0x046d,
            pid: 0xc52b,
            path: String::new(),
            usage_page: Some(1),
            usage: Some(2),
            input_report_bytes: Some(20),
            output_report_bytes: Some(20),
            feature_report_bytes: None,
        };
        assert!(!device.is_hidpp_candidate());
        device.usage_page = Some(0xff00);
        assert!(device.is_hidpp_candidate());
        assert_eq!(device.candidate_indices(), vec![1, 2, 3, 4, 5, 6]);
        device.vid = 0x1234;
        assert!(!device.is_hidpp_candidate());
        device.vid = 0x046d;
        device.pid = 0xb025;
        assert_eq!(device.candidate_indices(), vec![255]);
    }

    #[test]
    fn responses_ignore_notifications_and_other_slots_but_accept_protocol_errors() {
        let request = [0x11, 1, 4, 0x2a, 0, 0, 0];
        assert!(response_matches(&request, &[0x11, 1, 4, 0x2a, 0]));
        assert!(response_matches(&request, &[0x11, 1, 0xff, 4, 0x2a, 2]));
        assert!(response_matches(&request, &[0x10, 1, 0x8f, 4, 0x2a, 1]));
        assert!(!response_matches(&request, &[0x11, 2, 4, 0x2a, 0]));
        assert!(!response_matches(&request, &[0x11, 1, 4, 0x20, 0]));
        assert!(!response_matches(&request, &[0x11, 1, 4]));
        let direct = [0x11, 255, 4, 0x2a, 0, 0, 0];
        assert!(response_matches(&direct, &[0x11, 0, 4, 0x2a, 0]));
        assert!(!response_matches(&request, &[0x11, 0, 4, 0x2a, 0]));
    }
}

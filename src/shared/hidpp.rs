//! Experimental HID++ 2.0 device settings. Hardware validation is still required.
//!
//! Packet and feature facts were cross-checked against public protocol consumers:
//! https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/base.py
//! https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/hidpp20.py
//! https://github.com/pwr-Solaar/Solaar/blob/master/lib/logitech_receiver/settings_templates.py
//! This is an independent implementation, not a port of that project's implementation.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait Transport {
    /// Return a matching report, or a bounded timeout/error. No notifications here.
    fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScrollMode {
    Free,
    #[default]
    Ratchet,
    SmartShift,
}

#[derive(Clone, Debug, Serialize)]
pub struct BatteryStatus {
    pub percent: Option<u8>,
    pub level: Option<String>,
    pub status: String,
    pub voltage_mv: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DpiRange {
    pub min: u16,
    pub max: u16,
    pub step: u16,
}

#[derive(Clone, Debug, Serialize)]
pub struct ScrollStatus {
    pub mode: ScrollMode,
    /// Raw hardware disengagement threshold; 255 means locked ratchet.
    pub threshold: u8,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Capabilities {
    pub battery: bool,
    pub dpi: bool,
    pub scroll: bool,
    pub smart_shift: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct DeviceStatus {
    pub device_index: u8,
    pub protocol: String,
    pub name: Option<String>,
    pub battery: Option<BatteryStatus>,
    pub dpi: Option<u16>,
    pub dpi_choices: Vec<u16>,
    pub dpi_ranges: Vec<DpiRange>,
    pub scroll: Option<ScrollStatus>,
    pub capabilities: Capabilities,
    pub warnings: Vec<String>,
}

pub struct Client<T> {
    transport: T,
    device_index: u8,
    features: HashMap<u16, Option<u8>>,
    dpi_choices: Vec<u16>,
    dpi_feature: Option<u8>,
    scroll_feature: Option<(u8, bool, bool)>, // index, enhanced, torque supported
    next_software_id: u8,
    poisoned: bool,
}

struct DpiReading {
    current: u16,
    choices: Vec<u16>,
    ranges: Vec<DpiRange>,
    feature_index: u8,
}

impl<T: Transport> Client<T> {
    pub fn new(transport: T, device_index: u8) -> Self {
        Self {
            transport,
            device_index,
            features: HashMap::new(),
            dpi_choices: vec![],
            dpi_feature: None,
            scroll_feature: None,
            next_software_id: 0x0a,
            poisoned: false,
        }
    }

    fn request(
        &mut self,
        index: u8,
        function: u8,
        params: &[u8],
        needed: usize,
    ) -> Result<Vec<u8>, String> {
        if self.poisoned {
            return Err(
                "HID++ transaction failed; explicitly probe again before continuing".into(),
            );
        }
        if function > 15 || params.len() > 16 || needed > 16 {
            return Err("Invalid HID++ request size or function".into());
        }
        if self.device_index != 0xff && !(1..=6).contains(&self.device_index) {
            return Err("Only direct devices (255) or receiver slots 1–6 are supported".into());
        }
        let mut packet = [0u8; 20];
        let software_id = self.next_software_id;
        self.next_software_id = if software_id == 15 {
            1
        } else {
            software_id + 1
        };
        packet[..4].copy_from_slice(&[
            0x11,
            self.device_index,
            index,
            (function << 4) | software_id,
        ]);
        packet[4..4 + params.len()].copy_from_slice(params);
        let reply = match self.transport.exchange(&packet) {
            Ok(reply) => reply,
            Err(error) => {
                self.invalidate_transaction();
                return Err(error);
            }
        };
        if let Err(error) = validate_response(&packet, &reply, needed) {
            // These prefixes are produced only after the complete header and
            // echoed error request have passed validation. A device rejection
            // is therefore distinct from an uncorrelated/malformed response.
            if !error.starts_with("HID++ 1.0 error") && !error.starts_with("HID++ 2.0 error") {
                self.invalidate_transaction();
            }
            return Err(error);
        }
        let length = if reply[0] == 0x10 { 7 } else { 20 };
        Ok(reply[4..length].to_vec())
    }

    fn invalidate_transaction(&mut self) {
        self.poisoned = true;
        self.features.clear();
        self.dpi_feature = None;
        self.scroll_feature = None;
        self.dpi_choices.clear();
    }

    fn feature(&mut self, id: u16) -> Result<Option<u8>, String> {
        if let Some(index) = self.features.get(&id) {
            return Ok(*index);
        }
        let data = self.request(0, 0, &id.to_be_bytes(), 3)?;
        let index = (data[0] != 0).then_some(data[0]);
        self.features.insert(id, index);
        Ok(index)
    }

    pub fn probe(&mut self) -> Result<DeviceStatus, String> {
        // Invalidate before ping: a failed refresh must not leave old writable
        // feature indices alive. Software IDs continue advancing across probes.
        self.features.clear();
        self.dpi_feature = None;
        self.scroll_feature = None;
        self.dpi_choices.clear();
        self.poisoned = false;
        // ROOT.GetProtocolVersion: third parameter is an echoed probe marker.
        let ping = self.request(0, 1, &[0, 0, 0x73], 3)?;
        if ping[2] != 0x73 || ping[0] < 2 {
            return Err("Device did not confirm HID++ 2.0 or later".into());
        }
        let mut status = DeviceStatus {
            device_index: self.device_index,
            protocol: format!("{}.{}", ping[0], ping[1]),
            name: None,
            battery: None,
            dpi: None,
            dpi_choices: vec![],
            dpi_ranges: vec![],
            scroll: None,
            capabilities: Capabilities::default(),
            warnings: vec![],
        };
        match self.read_name() {
            Ok(value) => status.name = value,
            Err(error) => status.warnings.push(format!("Device name: {error}")),
        }
        match self.read_battery() {
            Ok(value) => {
                status.capabilities.battery = value.is_some();
                status.battery = value;
            }
            Err(error) => status.warnings.push(format!("Battery: {error}")),
        }
        match self.read_dpi() {
            Ok(Some(reading)) => {
                self.dpi_feature = Some(reading.feature_index);
                self.dpi_choices = reading.choices.clone();
                status.dpi = Some(reading.current);
                status.dpi_choices = reading.choices;
                status.dpi_ranges = reading.ranges;
                status.capabilities.dpi = true;
            }
            Ok(None) => (),
            Err(error) => status.warnings.push(format!("DPI: {error}")),
        }
        match self.discover_scroll() {
            Ok(value) => {
                status.capabilities.scroll = value.is_some();
                status.capabilities.smart_shift = value.is_some();
                status.scroll = value;
            }
            Err(error) => status.warnings.push(format!("Scroll: {error}")),
        }
        if self.poisoned {
            status.capabilities.dpi = false;
            status.capabilities.scroll = false;
            status.capabilities.smart_shift = false;
        }
        Ok(status)
    }

    fn read_name(&mut self) -> Result<Option<String>, String> {
        let Some(index) = self.feature(0x0005)? else {
            return Ok(None);
        };
        let len = self.request(index, 0, &[], 1)?[0] as usize;
        if len == 0 || len > 128 {
            return Err("Invalid device name length".into());
        }
        let mut name = Vec::with_capacity(len);
        while name.len() < len {
            let part = self.request(index, 1, &[name.len() as u8], 1)?;
            name.extend_from_slice(&part[..part.len().min(len - name.len())]);
        }
        String::from_utf8(name)
            .map(Some)
            .map_err(|_| "Invalid UTF-8 device name".into())
    }

    fn read_battery(&mut self) -> Result<Option<BatteryStatus>, String> {
        if let Some(index) = self.feature(0x1004)? {
            let data = self.request(index, 1, &[], 4)?;
            let level = match data[1] {
                8 => "full",
                4 => "good",
                2 => "low",
                1 => "critical",
                0 => "empty",
                _ => "unknown",
            };
            // Unified Battery reports may expose only a coarse level. Zero is
            // ambiguous; do not display a fabricated exact 0% in that case.
            return Ok(Some(BatteryStatus {
                percent: (1..=100).contains(&data[0]).then_some(data[0]),
                level: Some(level.into()),
                status: charge_status(data[2]),
                voltage_mv: None,
            }));
        }
        if let Some(index) = self.feature(0x1000)? {
            let data = self.request(index, 0, &[], 3)?;
            return Ok(Some(BatteryStatus {
                percent: (1..=100).contains(&data[0]).then_some(data[0]),
                level: None,
                status: charge_status(data[2]),
                voltage_mv: None,
            }));
        }
        if let Some(index) = self.feature(0x1001)? {
            let data = self.request(index, 0, &[], 3)?;
            let charging = data[2] & 0x80 != 0;
            let status = if charging && data[2] & 3 == 1 {
                "full"
            } else if charging && data[2] & 0x10 != 0 {
                "slow_recharge"
            } else if charging {
                "recharging"
            } else {
                "discharging"
            };
            return Ok(Some(BatteryStatus {
                percent: None,
                level: None,
                status: status.into(),
                voltage_mv: Some(u16::from_be_bytes([data[0], data[1]])),
            }));
        }
        Ok(None)
    }

    fn read_dpi(&mut self) -> Result<Option<DpiReading>, String> {
        let Some(index) = self.feature(0x2201)? else {
            return Ok(None);
        };
        if self.request(index, 0, &[], 1)?[0] == 0 {
            return Err("No DPI sensors".into());
        }
        // Sensor 0 only. 2201 uses a single report list; extended multi-page
        // lists belong to 2202, which this prototype intentionally does not use.
        let list = self.request(index, 1, &[0], 3)?;
        if list[0] != 0 {
            return Err("DPI list returned a different sensor".into());
        }
        let (choices, ranges) = parse_dpi_list(&list[1..])?;
        let data = self.request(index, 2, &[0], 5)?;
        if data[0] != 0 {
            return Err("DPI response returned a different sensor".into());
        }
        let current = u16::from_be_bytes([data[1], data[2]]);
        let current = if current == 0 {
            u16::from_be_bytes([data[3], data[4]])
        } else {
            current
        };
        if !choices.contains(&current) {
            return Err("Current DPI is outside the device's reported choices".into());
        }
        Ok(Some(DpiReading {
            current,
            choices,
            ranges,
            feature_index: index,
        }))
    }

    pub fn set_dpi(&mut self, dpi: u16) -> Result<(), String> {
        let index = self
            .dpi_feature
            .ok_or("DPI is unavailable; run a successful device probe first")?;
        if !self.dpi_choices.contains(&dpi) {
            return Err("DPI must match a device-supported value and step".into());
        }
        let [hi, lo] = dpi.to_be_bytes();
        self.request(index, 3, &[0, hi, lo], 0)?;
        let data = self.request(index, 2, &[0], 5)?;
        if data[0] != 0 || u16::from_be_bytes([data[1], data[2]]) != dpi {
            return Err("Device did not confirm the requested DPI".into());
        }
        Ok(())
    }

    fn discover_scroll(&mut self) -> Result<Option<ScrollStatus>, String> {
        let enhanced = self.feature(0x2111)?;
        let (index, is_enhanced, torque) = if let Some(index) = enhanced {
            let caps = self.request(index, 0, &[], 1)?;
            (index, true, caps[0] & 1 != 0)
        } else if let Some(index) = self.feature(0x2110)? {
            (index, false, false)
        } else {
            return Ok(None);
        };
        let data = self.request(
            index,
            if is_enhanced { 1 } else { 0 },
            &[],
            if torque { 3 } else { 2 },
        )?;
        let value = decode_scroll(&data)?;
        self.scroll_feature = Some((index, is_enhanced, torque));
        Ok(Some(value))
    }

    pub fn set_scroll(&mut self, mode: ScrollMode, threshold: u8) -> Result<(), String> {
        let (index, enhanced, torque) = self
            .scroll_feature
            .ok_or("Scroll settings are unavailable; probe the device first")?;
        if mode == ScrollMode::SmartShift && !(1..=49).contains(&threshold) {
            return Err(
                "SmartShift threshold must be 1–49; 50 and higher lock ratchet mode".into(),
            );
        }
        // Read immediately before changing settings; preserve Enhanced torque
        // rather than overwriting it with a hardcoded default.
        let current = self.request(
            index,
            if enhanced { 1 } else { 0 },
            &[],
            if torque { 3 } else { 2 },
        )?;
        decode_scroll(&current)?;
        let mut params = vec![
            if mode == ScrollMode::Free { 1 } else { 2 },
            if mode == ScrollMode::Ratchet {
                255
            } else if mode == ScrollMode::Free {
                current[1]
            } else {
                threshold
            },
        ];
        if enhanced && torque {
            params.push(current[2]);
        }
        self.request(index, if enhanced { 2 } else { 1 }, &params, 0)?;
        let checked = self.request(index, if enhanced { 1 } else { 0 }, &[], 2)?;
        let actual = decode_scroll(&checked)?;
        if actual.mode != mode || (mode == ScrollMode::SmartShift && actual.threshold != threshold)
        {
            return Err("Device did not confirm the requested scroll mode".into());
        }
        Ok(())
    }
}

fn charge_status(value: u8) -> String {
    match value {
        0 => "discharging".into(),
        1 => "recharging".into(),
        2 => "almost_full".into(),
        3 => "full".into(),
        4 => "slow_recharge".into(),
        5 => "invalid_battery".into(),
        6 => "thermal_error".into(),
        _ => format!("unknown_0x{value:02x}"),
    }
}

fn decode_scroll(data: &[u8]) -> Result<ScrollStatus, String> {
    if data.len() < 2 {
        return Err("Truncated SmartShift settings".into());
    }
    let mode = match data[0] {
        1 => ScrollMode::Free,
        2 if data[1] >= 50 => ScrollMode::Ratchet,
        2 => ScrollMode::SmartShift,
        _ => return Err("Unknown SmartShift wheel mode".into()),
    };
    Ok(ScrollStatus {
        mode,
        threshold: data[1],
    })
}

fn validate_response(request: &[u8], reply: &[u8], needed: usize) -> Result<(), String> {
    if reply.len() < 4 {
        return Err("Truncated HID++ header".into());
    }
    let length = match reply[0] {
        0x10 => 7,
        0x11 => 20,
        _ => return Err("Unexpected HID report ID".into()),
    };
    if reply.len() < length {
        return Err("Truncated HID++ report".into());
    }
    // Some BLE devices return 0 for the direct-device address 255. Only accept
    // this documented alias; never accept another receiver slot's response.
    if reply[1] != request[1] && !(request[1] == 255 && reply[1] == 0) {
        return Err("HID++ response belongs to a different device".into());
    }
    if reply[2] == 0x8f || reply[2] == 0xff {
        if reply[3] != request[2] || reply[4] != request[3] {
            return Err("Mismatched HID++ error response".into());
        }
        return Err(format!(
            "HID++ {} error 0x{:02x}",
            if reply[2] == 0xff { "2.0" } else { "1.0" },
            reply[5]
        ));
    }
    if reply[2..4] != request[2..4] {
        return Err("HID++ feature/function/software ID mismatch".into());
    }
    if length - 4 < needed {
        return Err("HID++ response has insufficient parameters".into());
    }
    Ok(())
}

fn parse_dpi_list(data: &[u8]) -> Result<(Vec<u16>, Vec<DpiRange>), String> {
    let words: Vec<u16> = data
        .chunks_exact(2)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .collect();
    let mut values = vec![];
    let mut ranges = vec![];
    let mut i = 0;
    let mut terminated = false;
    while i < words.len() {
        let word = words[i];
        if word == 0 {
            terminated = true;
            break;
        }
        if word & 0xe000 == 0xe000 {
            let step = word & 0x1fff;
            let min = *values.last().ok_or("DPI range lacks a starting value")?;
            let max = *words.get(i + 1).ok_or("DPI range lacks an ending value")?;
            if step == 0 || max <= min || max >= 0xe000 || (max - min) % step != 0 {
                return Err("Invalid DPI range/step".into());
            }
            ranges.push(DpiRange { min, max, step });
            let mut next = min as u32 + step as u32;
            while next <= max as u32 {
                values.push(next as u16);
                next += step as u32;
            }
            i += 2;
        } else {
            if values.last().is_some_and(|last| word <= *last) {
                return Err("Unordered DPI list".into());
            }
            values.push(word);
            i += 1;
        }
        if values.len() > 8192 {
            return Err("DPI list exceeds the supported safety bound".into());
        }
    }
    if values.is_empty() || !terminated {
        return Err("Empty or unterminated DPI list".into());
    }
    Ok((values, ranges))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    struct Mock {
        transactions: VecDeque<(Vec<u8>, Vec<u8>)>,
    }
    impl Mock {
        fn new(mut transactions: VecDeque<(Vec<u8>, Vec<u8>)>) -> Self {
            for (ordinal, (request, reply)) in transactions.iter_mut().enumerate() {
                let software_id = ((9 + ordinal) % 15 + 1) as u8;
                request[3] = (request[3] & 0xf0) | software_id;
                reply[3] = (reply[3] & 0xf0) | software_id;
            }
            Self { transactions }
        }
    }
    impl Transport for Mock {
        fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, String> {
            let (expected, response) = self
                .transactions
                .pop_front()
                .expect("Unexpected transaction");
            assert_eq!(request, expected);
            Ok(response)
        }
    }
    fn packet(index: u8, function: u8, data: &[u8]) -> Vec<u8> {
        let mut p = vec![0; 20];
        p[..4].copy_from_slice(&[0x11, 255, index, (function << 4) | 0x0a]);
        p[4..4 + data.len()].copy_from_slice(data);
        p
    }
    fn pair(index: u8, function: u8, request: &[u8], reply: &[u8]) -> (Vec<u8>, Vec<u8>) {
        (
            packet(index, function, request),
            packet(index, function, reply),
        )
    }
    #[test]
    fn validates_headers_errors_and_lengths() {
        let request = packet(7, 2, &[0]);
        let valid = packet(7, 2, &[0, 3, 232, 3, 232]);
        assert!(validate_response(&request, &valid, 5).is_ok());
        for (offset, value) in [(0, 0x12), (1, 2), (2, 8), (3, 0x2b)] {
            let mut changed = valid.clone();
            changed[offset] = value;
            assert!(validate_response(&request, &changed, 5).is_err());
        }
        assert!(validate_response(&request, &valid[..19], 5).is_err());
        assert!(validate_response(&request, &[0x10, 255, 7, 0x2a, 0, 0, 0], 5).is_err());
        for error in [0xff, 0x8f] {
            let response = [0x10, 255, error, 7, 0x2a, 2, 0];
            assert!(
                validate_response(&request, &response, 0)
                    .unwrap_err()
                    .contains("0x02")
            );
        }
        let mut ble = valid;
        ble[1] = 0;
        assert!(validate_response(&request, &ble, 5).is_ok());
    }
    #[test]
    fn dpi_range_fixture_and_adversarial_steps() {
        let (values, ranges) = parse_dpi_list(&[0, 200, 0xe0, 50, 0x0f, 0xa0, 0, 0]).unwrap();
        assert_eq!(values.first(), Some(&200));
        assert_eq!(values.last(), Some(&4000));
        assert_eq!(values.len(), 77);
        assert_eq!(
            ranges,
            vec![DpiRange {
                min: 200,
                max: 4000,
                step: 50
            }]
        );
        for bad in [
            &[0, 200, 0xe0, 0, 0x0f, 0xa0, 0, 0][..],
            &[0, 200, 0xe0, 51, 0x0f, 0xa0, 0, 0],
            &[0xe0, 50, 0x0f, 0xa0, 0, 0],
            &[0, 200, 0, 100, 0, 0],
            &[0, 200, 0, 250],
        ] {
            assert!(parse_dpi_list(bad).is_err());
        }
    }
    #[test]
    fn unsupported_features_disable_writes_and_probe_is_read_only() {
        let mut transactions = VecDeque::from([pair(0, 1, &[0, 0, 0x73], &[4, 5, 0x73])]);
        for feature in [0x0005u16, 0x1004, 0x1000, 0x1001, 0x2201, 0x2111, 0x2110] {
            transactions.push_back(pair(0, 0, &feature.to_be_bytes(), &[0, 0, 0]));
        }
        let mut client = Client::new(Mock::new(transactions), 255);
        let status = client.probe().unwrap();
        assert!(!status.capabilities.dpi);
        assert!(!status.capabilities.scroll);
        assert!(status.battery.is_none());
        assert!(status.warnings.is_empty());
        assert!(client.set_dpi(1000).is_err());
        assert!(client.set_scroll(ScrollMode::Free, 1).is_err());
        assert!(client.transport.transactions.is_empty());
    }
    #[test]
    fn full_read_probe_discovers_supported_hardware_settings() {
        let transactions = VecDeque::from([
            pair(0, 1, &[0, 0, 0x73], &[4, 5, 0x73]),
            pair(0, 0, &[0, 5], &[0, 0, 0]),
            pair(0, 0, &[0x10, 4], &[3, 0, 0]),
            pair(3, 1, &[], &[80, 4, 0, 0]),
            pair(0, 0, &[0x22, 1], &[7, 0, 0]),
            pair(7, 0, &[], &[1]),
            pair(7, 1, &[0], &[0, 0, 200, 0xe0, 50, 0x0f, 0xa0, 0, 0]),
            pair(7, 2, &[0], &[0, 3, 232, 3, 232]),
            pair(0, 0, &[0x21, 0x11], &[9, 0, 0]),
            pair(9, 0, &[], &[1, 50, 45]),
            pair(9, 1, &[], &[2, 30, 45]),
        ]);
        let mut client = Client::new(Mock::new(transactions), 255);
        let status = client.probe().unwrap();
        assert_eq!(status.battery.unwrap().percent, Some(80));
        assert_eq!(status.dpi, Some(1000));
        assert_eq!(status.dpi_choices.len(), 77);
        assert_eq!(status.scroll.unwrap().mode, ScrollMode::SmartShift);
        assert!(status.capabilities.dpi && status.capabilities.smart_shift);
        assert!(status.warnings.is_empty());
        assert!(client.transport.transactions.is_empty());
    }
    #[test]
    fn invalid_dpi_never_reaches_transport() {
        let mut client = Client::new(
            Mock {
                transactions: VecDeque::new(),
            },
            255,
        );
        client.dpi_feature = Some(7);
        client.dpi_choices = vec![200, 250, 300];
        assert!(client.set_dpi(225).is_err());
        assert!(client.set_dpi(0).is_err());
    }
    #[test]
    fn enhanced_scroll_preserves_torque_and_confirms_actual_setting() {
        let transactions = VecDeque::from([
            pair(9, 1, &[], &[2, 30, 45]),
            pair(9, 2, &[2, 15, 45], &[]),
            pair(9, 1, &[], &[2, 15, 45]),
        ]);
        let mut client = Client::new(Mock::new(transactions), 255);
        client.scroll_feature = Some((9, true, true));
        client.set_scroll(ScrollMode::SmartShift, 15).unwrap();
        assert!(client.set_scroll(ScrollMode::SmartShift, 50).is_err());
        assert!(client.transport.transactions.is_empty());
    }
    #[test]
    fn dpi_write_is_big_endian_and_verified() {
        let transactions = VecDeque::from([
            pair(7, 3, &[0, 0x03, 0xe8], &[]),
            pair(7, 2, &[0], &[0, 0x03, 0xe8, 0x03, 0xe8]),
        ]);
        let mut client = Client::new(Mock::new(transactions), 255);
        client.dpi_feature = Some(7);
        client.dpi_choices = vec![1000];
        client.set_dpi(1000).unwrap();
        assert!(client.transport.transactions.is_empty());
    }
    #[test]
    fn late_feature_reply_is_rejected_and_client_cannot_continue_writing() {
        struct LateReply {
            calls: usize,
        }
        impl Transport for LateReply {
            fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, String> {
                self.calls += 1;
                assert_eq!(request[3] & 15, 9 + self.calls as u8);
                // Both replies carry the old transaction's software ID a.
                Ok(packet(0, 0, &[7, 0, 0]))
            }
        }
        let mut client = Client::new(LateReply { calls: 0 }, 255);
        assert_eq!(client.feature(0x0005).unwrap(), Some(7));
        assert!(client.feature(0x1004).unwrap_err().contains("mismatch"));
        assert!(client.features.is_empty());
        assert!(client.feature(0x2201).is_err());
        assert!(client.set_dpi(1000).is_err());
        assert_eq!(client.transport.calls, 2);
    }
    #[test]
    fn failed_refresh_invalidates_previous_capabilities_before_ping() {
        let mut client = Client::new(
            Mock::new(VecDeque::from([pair(0, 1, &[0, 0, 0x73], &[1, 0, 0x73])])),
            255,
        );
        client.dpi_feature = Some(7);
        client.dpi_choices = vec![1000];
        client.scroll_feature = Some((9, true, true));
        assert!(client.probe().is_err());
        assert!(client.set_dpi(1000).is_err());
        assert!(client.set_scroll(ScrollMode::Free, 1).is_err());
        assert_eq!(client.next_software_id, 11);
    }
    #[test]
    fn software_id_wraps_without_using_notification_id_zero() {
        let transactions = (0..20).map(|_| pair(0, 0, &[0, 5], &[0, 0, 0])).collect();
        let mut client = Client::new(Mock::new(transactions), 255);
        for _ in 0..20 {
            client.request(0, 0, &[0, 5], 3).unwrap();
            assert!((1..=15).contains(&client.next_software_id));
        }
        assert!(client.transport.transactions.is_empty());
    }
    #[test]
    fn correlated_device_rejection_does_not_poison_later_reads() {
        struct DeviceRejection {
            calls: usize,
        }
        impl Transport for DeviceRejection {
            fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, String> {
                self.calls += 1;
                if self.calls == 1 {
                    Ok(vec![0x10, request[1], 0xff, request[2], request[3], 6, 0])
                } else {
                    let mut reply = request.to_vec();
                    reply[4] = 9;
                    Ok(reply)
                }
            }
        }
        let mut client = Client::new(DeviceRejection { calls: 0 }, 255);
        assert!(client.feature(0x0005).unwrap_err().contains("0x06"));
        assert!(!client.poisoned);
        assert_eq!(client.feature(0x1004).unwrap(), Some(9));
    }
    #[test]
    fn transport_timeout_disables_all_cached_writes() {
        struct Timeout;
        impl Transport for Timeout {
            fn exchange(&mut self, _: &[u8]) -> Result<Vec<u8>, String> {
                Err("timeout".into())
            }
        }
        let mut client = Client::new(Timeout, 255);
        client.dpi_feature = Some(7);
        client.dpi_choices = vec![1000];
        client.scroll_feature = Some((9, true, true));
        assert!(client.request(0, 0, &[0, 5], 3).is_err());
        assert!(client.poisoned);
        assert!(client.set_dpi(1000).is_err());
        assert!(client.set_scroll(ScrollMode::Free, 1).is_err());
    }
}

//! Bluetooth device management.
//!
//! Linux: full control via `bluetoothctl`. Windows: list paired devices and
//! toggle the adapter via PnP (connect/disconnect is out of scope — Windows has
//! no simple CLI for it).
//!
//! All subprocess calls are async (tokio) and bounded by a timeout, so a
//! hung `bluetoothctl` or a missing `bluetoothd` cannot lock the UI. Each
//! failure mode is reported via the typed [`BtError`] enum; the frontend
//! receives a human-readable string (Tauri serialises errors via `Display`).

use serde::Serialize;
use std::time::Duration;

#[derive(Clone, Debug, Serialize)]
pub struct BtDevice {
    pub mac: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    pub icon: String, // audio-card, input-keyboard, phone, ...
    pub battery: Option<u8>, // 0-100, when the device reports it
    pub rssi: Option<i16>,   // signal strength in dBm, when in range
}

#[derive(Clone, Debug, Serialize)]
pub struct BtState {
    pub available: bool,
    pub powered: bool,
    pub devices: Vec<BtDevice>,
}

// `DaemonDown`, `DeviceNotFound`, `NotPowered` are intentionally part of the
// public error API even though the current code paths don't return them yet —
// they're wired in for the upcoming Phase 2 (scan/unpair) and Phase 3 (typed
// error mapping in the frontend).
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum BtError {
    #[error("bluetoothctl not found — install bluez-utils")]
    NotInstalled,
    #[error("bluetoothd is not running")]
    DaemonDown,
    #[error("device {0} not found")]
    DeviceNotFound(String),
    #[error("adapter is not powered")]
    NotPowered,
    #[error("operation timed out after {0}s")]
    Timeout(u64),
    #[error("bluetoothctl: {0}")]
    Backend(String),
}

// Tauri commands return `Result<T, E>` where E must be `Serialize`. We surface
// errors to the frontend as a plain string (the `Display` message), which
// matches the previous `String` return type and keeps the TS contract stable.
impl serde::Serialize for BtError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

/// Time bound for short read-only queries (state, info, list).
const QUERY_TIMEOUT: Duration = Duration::from_secs(3);
/// Time bound for actions that may legitimately take a while (connect scans,
/// pairing, power-cycle). 8 s is generous; a healthy bluetoothctl finishes
/// most actions in well under 2 s.
const ACTION_TIMEOUT: Duration = Duration::from_secs(8);

// ── Linux (bluetoothctl) ──────────────────────────────────────────────────────

#[cfg(not(windows))]
pub use unix_impl::{
    connect, disconnect, is_available, pair, remove, scan, set_power, set_trust, state,
};

#[cfg(not(windows))]
mod unix_impl {
    use super::{BtDevice, BtError, BtState, ACTION_TIMEOUT, QUERY_TIMEOUT};
    use std::process::Stdio;
    use std::time::Duration;
    use tokio::process::Command;

    /// Cheap availability check: is the `bluetoothctl` binary on PATH?
    /// Use a tight 1 s timeout — we don't want a slow fork to delay startup.
    pub async fn is_available() -> bool {
        match tokio::time::timeout(
            Duration::from_secs(1),
            Command::new("bluetoothctl").arg("--version").output(),
        )
        .await
        {
            Ok(Ok(o)) => o.status.success(),
            _ => false,
        }
    }

    pub async fn state() -> BtState {
        if !is_available().await {
            return BtState {
                available: false,
                powered: false,
                devices: Vec::new(),
            };
        }
        // Probe the daemon. `bluetoothctl show` prints the default controller
        // (or "No default controller available" if bluetoothd is down).
        let show = match run(&["show"], QUERY_TIMEOUT).await {
            Ok(t) => t,
            Err(BtError::NotInstalled) => {
                return BtState {
                    available: false,
                    powered: false,
                    devices: Vec::new(),
                };
            }
            Err(BtError::Timeout(_)) | Err(BtError::Backend(_)) => {
                // Binary present but unusable → daemon is likely down.
                return BtState {
                    available: true,
                    powered: false,
                    devices: Vec::new(),
                };
            }
            Err(_) => {
                return BtState {
                    available: true,
                    powered: false,
                    devices: Vec::new(),
                };
            }
        };

        let (has_controller, powered) = parse_show(&show);
        if !has_controller {
            return BtState {
                available: true,
                powered: false,
                devices: Vec::new(),
            };
        }

        let devices = match devices().await {
            Ok(ds) => ds,
            Err(_) => Vec::new(), // surface partial state instead of failing
        };

        BtState {
            available: true,
            powered,
            devices,
        }
    }

    /// Returns `(has_controller, powered)`. `show` is the stdout of
    /// `bluetoothctl show`.
    fn parse_show(show: &str) -> (bool, bool) {
        if show.is_empty() {
            return (false, false);
        }
        // bluetoothd not running: every relevant field shows "not available".
        if show.contains("not available") {
            return (false, false);
        }
        let powered = show
            .lines()
            .any(|l| l.trim().starts_with("Powered: yes"));
        (true, powered)
    }

    async fn devices() -> Result<Vec<BtDevice>, BtError> {
        let list = run(&["devices"], QUERY_TIMEOUT).await?;
        let macs: Vec<String> = list
            .lines()
            .filter_map(|line| {
                // "Device AA:BB:CC:DD:EE:FF Name Here"
                let parts: Vec<&str> = line.trim().splitn(3, ' ').collect();
                if parts.len() >= 2 && parts[0] == "Device" {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            })
            .collect();

        // Run all `info` calls in parallel. Each is bounded by QUERY_TIMEOUT.
        let handles: Vec<_> = macs
            .iter()
            .map(|mac| {
                let m = mac.clone();
                tokio::spawn(async move { info(&m).await })
            })
            .collect();
        let mut out = Vec::with_capacity(handles.len());
        for h in handles {
            match h.await {
                Ok(Some(d)) => out.push(d),
                _ => {} // skip individual failures
            }
        }
        Ok(out)
    }

    async fn info(mac: &str) -> Option<BtDevice> {
        let text = run(&["info", mac], QUERY_TIMEOUT).await.ok()?;
        Some(parse_info(&text, mac))
    }

    /// Parse the stdout of `bluetoothctl info <mac>`. Pure function — public
    /// to the module for unit tests.
    pub(super) fn parse_info(text: &str, mac: &str) -> BtDevice {
        let field = |key: &str| -> Option<String> {
            text.lines()
                .find_map(|l| l.trim().strip_prefix(key).map(|v| v.trim().to_string()))
        };
        let yes = |k: &str| field(k).map(|v| v == "yes").unwrap_or(false);
        BtDevice {
            mac: mac.to_string(),
            name: field("Name:").unwrap_or_else(|| mac.to_string()),
            paired: yes("Paired:"),
            connected: yes("Connected:"),
            trusted: yes("Trusted:"),
            icon: field("Icon:").unwrap_or_default(),
            battery: field("Battery Percentage:").and_then(|v| parse_paren_num(&v)),
            rssi: field("RSSI:").and_then(|v| parse_signed(&v)),
        }
    }

    /// Battery prints as `0x55 (85)` — take the decimal in parens, falling back
    /// to a hex `0xNN` or a bare decimal.
    fn parse_paren_num(v: &str) -> Option<u8> {
        if let Some(inner) = paren_inner(v) {
            return inner.parse::<u8>().ok();
        }
        if let Some(hex) = v.trim().strip_prefix("0x") {
            return u8::from_str_radix(hex, 16).ok();
        }
        v.trim().parse::<u8>().ok()
    }

    /// RSSI comes as `-50` or `0xffffffce (-50)` — prefer the parenthesised
    /// signed decimal.
    fn parse_signed(v: &str) -> Option<i16> {
        if let Some(inner) = paren_inner(v) {
            return inner.parse::<i16>().ok();
        }
        v.split_whitespace().next().and_then(|t| t.parse::<i16>().ok())
    }

    fn paren_inner(v: &str) -> Option<&str> {
        let start = v.find('(')?;
        let end = v[start + 1..].find(')')?;
        Some(v[start + 1..start + 1 + end].trim())
    }

    /// Pair (bond) with a discovered device. Works for "Just Works" devices
    /// (most headsets, mice, keyboards); a device that requires a PIN/passkey
    /// needs an interactive agent and will surface a backend error here.
    pub async fn pair(mac: &str) -> Result<(), BtError> {
        run_cmd(&["pair", mac], ACTION_TIMEOUT).await
    }

    pub async fn connect(mac: &str) -> Result<(), BtError> {
        run_cmd(&["connect", mac], ACTION_TIMEOUT).await
    }

    pub async fn disconnect(mac: &str) -> Result<(), BtError> {
        run_cmd(&["disconnect", mac], ACTION_TIMEOUT).await
    }

    pub async fn set_power(on: bool) -> Result<(), BtError> {
        run_cmd(&["power", if on { "on" } else { "off" }], ACTION_TIMEOUT).await
    }

    pub async fn set_trust(mac: &str, trust: bool) -> Result<(), BtError> {
        run_cmd(&[if trust { "trust" } else { "untrust" }, mac], ACTION_TIMEOUT).await
    }

    /// Unpair (forget) a device. The MAC is removed from the known-devices
    /// list; the device must be re-paired to be used again.
    pub async fn remove(mac: &str) -> Result<(), BtError> {
        run_cmd(&["remove", mac], ACTION_TIMEOUT).await
    }

    /// Run a discovery scan for `secs` seconds and return every device that
    /// showed up during the window (paired or not). The Bluetooth spec keeps
    /// the radio active after a scan, so we always send `scan off` afterwards
    /// even on error paths.
    pub async fn scan(secs: u64) -> Result<Vec<BtDevice>, BtError> {
        // Bounds: 1 s minimum (anything shorter is unreliable), 30 s max to
        // prevent runaway radio activity.
        let secs = secs.clamp(1, 30);
        let _ = run_cmd(&["scan", "on"], Duration::from_secs(2)).await;
        tokio::time::sleep(Duration::from_secs(secs)).await;
        let _ = run_cmd(&["scan", "off"], Duration::from_secs(2)).await;

        // Reuse the standard state pipeline to surface anything new.
        let mut out = state().await;
        // Stable order: connected first, then by name.
        out.devices.sort_by(|a, b| {
            b.connected
                .cmp(&a.connected)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(out.devices)
    }

    async fn run(args: &[&str], timeout: Duration) -> Result<String, BtError> {
        let fut = Command::new("bluetoothctl")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        let out = match tokio::time::timeout(timeout, fut).await {
            Ok(Ok(o)) => o,
            Ok(Err(_)) => return Err(BtError::NotInstalled),
            Err(_) => return Err(BtError::Timeout(timeout.as_secs())),
        };
        if !out.status.success() {
            return Err(BtError::Backend(format!(
                "bluetoothctl {} failed (exit {:?})",
                args.join(" "),
                out.status.code()
            )));
        }
        String::from_utf8(out.stdout)
            .map_err(|e| BtError::Backend(format!("invalid utf-8: {e}")))
    }

    async fn run_cmd(args: &[&str], timeout: Duration) -> Result<(), BtError> {
        // bluetoothctl returns success even when the action reports an error in
        // text, so surface stdout on obvious failures.
        let fut = Command::new("bluetoothctl")
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output();
        let out = match tokio::time::timeout(timeout, fut).await {
            Ok(Ok(o)) => o,
            Ok(Err(_)) => return Err(BtError::NotInstalled),
            Err(_) => return Err(BtError::Timeout(timeout.as_secs())),
        };
        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains("Failed") || stdout.contains("not available") {
            let line = stdout
                .lines()
                .find(|l| l.contains("Failed") || l.contains("not available"))
                .unwrap_or("operation failed");
            return Err(BtError::Backend(line.trim().to_string()));
        }
        if out.status.success() {
            Ok(())
        } else {
            Err(BtError::Backend(format!(
                "bluetoothctl {} failed (exit {:?})",
                args.join(" "),
                out.status.code()
            )))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        // Real `bluetoothctl show` output, recorded on a live system. Keep
        // the format identical to BlueZ 5.x so the parser stays current.
        const SHOW_OK: &str = "\
Controller AA:BB:CC:DD:EE:FF MyPC [default]
\tName: MyPC
\tPowered: yes
\tDiscoverable: no
\tPairable: yes
";

        const SHOW_OFF: &str = "\
Controller AA:BB:CC:DD:EE:FF MyPC [default]
\tName: MyPC
\tPowered: no
";

        const SHOW_DOWN: &str = "\
Controller AA:BB:CC:DD:EE:FF not available
\tName: not available
\tPowered: no
";

        #[test]
        fn show_powered_yes() {
            let (has_ctrl, on) = parse_show(SHOW_OK);
            assert!(has_ctrl);
            assert!(on);
        }

        #[test]
        fn show_powered_no() {
            let (has_ctrl, on) = parse_show(SHOW_OFF);
            assert!(has_ctrl);
            assert!(!on);
        }

        #[test]
        fn show_daemon_down() {
            let (has_ctrl, on) = parse_show(SHOW_DOWN);
            assert!(!has_ctrl, "should treat 'not available' as no controller");
            assert!(!on);
        }

        #[test]
        fn show_empty_is_down() {
            let (has_ctrl, on) = parse_show("");
            assert!(!has_ctrl);
            assert!(!on);
        }

        // Real `bluetoothctl info <mac>` output (truncated).
        const INFO_OK: &str = "\
Device AA:BB:CC:DD:EE:FF
\tName: Sony WH-1000XM4
\tPaired: yes
\tTrusted: yes
\tConnected: yes
\tIcon: audio-card
\tBattery Percentage: 0x55 (85)
\tRSSI: -42
";

        #[test]
        fn info_parses_all_fields() {
            let d = parse_info(INFO_OK, "AA:BB:CC:DD:EE:FF");
            assert_eq!(d.mac, "AA:BB:CC:DD:EE:FF");
            assert_eq!(d.name, "Sony WH-1000XM4");
            assert!(d.paired);
            assert!(d.trusted);
            assert!(d.connected);
            assert_eq!(d.icon, "audio-card");
            assert_eq!(d.battery, Some(85));
            assert_eq!(d.rssi, Some(-42));
        }

        #[test]
        fn info_falls_back_to_mac_for_missing_name() {
            let d = parse_info("Device AA:BB:CC:DD:EE:FF\n", "AA:BB:CC:DD:EE:FF");
            assert_eq!(d.name, "AA:BB:CC:DD:EE:FF");
            assert!(!d.paired);
            assert!(!d.connected);
            assert_eq!(d.battery, None);
            assert_eq!(d.rssi, None);
        }

        #[test]
        fn battery_parses_hex_only_form() {
            // Some BlueZ builds omit the parenthesised decimal.
            let d = parse_info("Device X\n\tBattery Percentage: 0x64\n", "X");
            assert_eq!(d.battery, Some(100));
        }
    }
}

// ── Windows (PnP) ─────────────────────────────────────────────────────────────

#[cfg(windows)]
pub use win_impl::{
    connect, disconnect, is_available, pair, remove, scan, set_power, set_trust, state,
};

#[cfg(windows)]
mod win_impl {
    use super::{BtDevice, BtError, BtState};
    use std::time::Duration;
    use tokio::process::Command;

    const PS_TIMEOUT: Duration = Duration::from_secs(5);

    async fn ps(script: &str, timeout: Duration) -> Result<String, BtError> {
        let fut = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output();
        let out = match tokio::time::timeout(timeout, fut).await {
            Ok(Ok(o)) => o,
            Ok(Err(_)) => return Err(BtError::NotInstalled),
            Err(_) => return Err(BtError::Timeout(timeout.as_secs())),
        };
        if !out.status.success() {
            return Err(BtError::Backend(format!(
                "powershell exit {:?}",
                out.status.code()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// A paired remote device is enumerated under BTHENUM/BTHLE; anything else of
    /// class Bluetooth (the USB/PCI radio and the MS enumerators) is "infrastructure".
    pub fn is_paired_device(instance_id: &str) -> bool {
        let id = instance_id.to_ascii_uppercase();
        id.starts_with("BTHENUM") || id.starts_with("BTHLE")
    }

    pub async fn is_available() -> bool {
        ps(
            "(Get-PnpDevice -Class Bluetooth -PresentOnly -ErrorAction SilentlyContinue | Measure-Object).Count",
            PS_TIMEOUT,
        )
        .await
        .ok()
        .and_then(|s| s.trim().parse::<i64>().ok())
        .map(|n| n > 0)
        .unwrap_or(false)
    }

    pub async fn state() -> BtState {
        let json = match ps(
            "Get-PnpDevice -Class Bluetooth -PresentOnly -ErrorAction SilentlyContinue | \
             Select-Object InstanceId,FriendlyName,Status | ConvertTo-Json -Compress",
            PS_TIMEOUT,
        )
        .await
        {
            Ok(j) if !j.trim().is_empty() => j,
            _ => {
                return BtState {
                    available: false,
                    powered: false,
                    devices: Vec::new(),
                }
            }
        };

        let value: serde_json::Value = match serde_json::from_str(&json) {
            Ok(v) => v,
            Err(_) => {
                return BtState {
                    available: false,
                    powered: false,
                    devices: Vec::new(),
                }
            }
        };
        let items = match value {
            serde_json::Value::Array(a) => a,
            obj @ serde_json::Value::Object(_) => vec![obj],
            _ => Vec::new(),
        };

        let mut devices: Vec<BtDevice> = Vec::new();
        let mut powered = false;
        let mut any = false;
        for it in items {
            any = true;
            let get = |k: &str| it.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let instance = get("InstanceId");
            let status = get("Status");
            let ok = status.eq_ignore_ascii_case("OK");

            if is_paired_device(&instance) {
                let name = {
                    let n = get("FriendlyName");
                    if n.is_empty() { instance.clone() } else { n }
                };
                let mac = mac_from_instance(&instance).unwrap_or_else(|| instance.clone());
                merge_device(&mut devices, mac, name, ok);
            } else if ok {
                // A radio/adapter that's OK means Bluetooth is powered on.
                powered = true;
            }
        }

        BtState {
            available: any,
            powered,
            devices,
        }
    }

    /// Toggle every Bluetooth radio/adapter (not the paired-device entries).
    /// Requires Administrator, so route through UAC.
    pub async fn set_power(on: bool) -> Result<(), BtError> {
        let verb = if on { "Enable-PnpDevice" } else { "Disable-PnpDevice" };
        let action = format!(
            "Get-PnpDevice -Class Bluetooth | Where-Object {{ $_.InstanceId -notlike 'BTHENUM*' \
             -and $_.InstanceId -notlike 'BTHLE*' }} | {verb} -Confirm:$false"
        );
        crate::winutil::run_elevated(&action)
            .map_err(|e| BtError::Backend(e))
    }

    pub async fn connect(_mac: &str) -> Result<(), BtError> {
        Err(BtError::Backend(
            "Connecting Bluetooth devices isn't supported on Windows yet — pair/connect from \
             Windows Settings. You can still toggle the adapter here."
                .into(),
        ))
    }

    pub async fn disconnect(_mac: &str) -> Result<(), BtError> {
        Err(BtError::Backend(
            "Disconnecting Bluetooth devices isn't supported on Windows yet — use Windows Settings."
                .into(),
        ))
    }

    pub async fn set_trust(_mac: &str, _trust: bool) -> Result<(), BtError> {
        Err(BtError::Backend(
            "Trust management isn't applicable on Windows.".into(),
        ))
    }

    pub async fn remove(_mac: &str) -> Result<(), BtError> {
        Err(BtError::Backend(
            "Removing paired devices isn't supported on Windows yet — use \
             Windows Settings → Bluetooth & devices → Devices."
                .into(),
        ))
    }

    pub async fn pair(_mac: &str) -> Result<(), BtError> {
        Err(BtError::Backend(
            "Pairing new devices isn't supported on Windows yet — pair from \
             Windows Settings → Bluetooth & devices."
                .into(),
        ))
    }

    pub async fn scan(_secs: u64) -> Result<Vec<BtDevice>, BtError> {
        Err(BtError::Backend(
            "Scanning for new devices isn't supported on Windows yet — pair \
             from Windows Settings → Bluetooth & devices."
                .into(),
        ))
    }

    /// Extract a 12-hex MAC from a Bluetooth PnP instance id. Works across the
    /// device form (`...\DEV_AABBCCDDEEFF\...`), the profile-transport form
    /// (`...&AABBCCDDEEFF_C00000000`) and the BLE form
    /// (`BTHLEDEVICE\{guid}_AABBCCDDEEFF\...`). GUID braces are stripped first so
    /// the 12-hex tail of the Bluetooth base UUID can't be mistaken for a MAC.
    pub(crate) fn mac_from_instance(instance: &str) -> Option<String> {
        let cleaned = strip_braces(&instance.to_ascii_uppercase());
        // First run of *exactly* 12 hex digits.
        let mut runs: Vec<String> = Vec::new();
        let mut cur = String::new();
        for c in cleaned.chars() {
            if c.is_ascii_hexdigit() {
                cur.push(c);
            } else if !cur.is_empty() {
                runs.push(std::mem::take(&mut cur));
            }
        }
        if !cur.is_empty() {
            runs.push(cur);
        }
        runs.into_iter().find(|r| r.len() == 12).map(|r| fmt_mac(&r))
    }

    /// Replace `{...}` GUID spans with spaces (separators) so their hex doesn't
    /// merge with surrounding tokens.
    fn strip_braces(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut depth = 0u32;
        for c in s.chars() {
            match c {
                '{' => {
                    depth += 1;
                    out.push(' ');
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    out.push(' ');
                }
                _ => out.push(if depth == 0 { c } else { ' ' }),
            }
        }
        out
    }

    fn fmt_mac(hex: &str) -> String {
        hex.as_bytes()
            .chunks(2)
            .map(|c| String::from_utf8_lossy(c).into_owned())
            .collect::<Vec<_>>()
            .join(":")
    }

    /// A profile/transport sub-node name (AVRCP, A2DP, hands-free…), not the
    /// device's main entry — used to prefer the cleaner name when merging.
    pub(crate) fn is_subprofile(name: &str) -> bool {
        let n = name.to_lowercase();
        ["avrcp", "transporte", "a2dp", "avdtp", "hands-free", "handsfree", "hfp", "transport"]
            .iter()
            .any(|k| n.contains(k))
    }

    /// Add a device, or merge into an existing one with the same MAC (collapsing
    /// AVRCP/A2DP sub-profile entries into a single device and OR-ing connected).
    fn merge_device(devices: &mut Vec<BtDevice>, mac: String, name: String, connected: bool) {
        if let Some(d) = devices.iter_mut().find(|d| d.mac == mac) {
            d.connected |= connected;
            let replace = (is_subprofile(&d.name) && !is_subprofile(&name))
                || (!is_subprofile(&d.name) && !is_subprofile(&name) && name.len() < d.name.len());
            if replace {
                d.icon = icon_for(&name);
                d.name = name;
            }
        } else {
            devices.push(BtDevice {
                icon: icon_for(&name),
                mac,
                name,
                paired: true,
                connected,
                trusted: false,
                battery: None,
                rssi: None,
            });
        }
    }

    fn icon_for(name: &str) -> String {
        let n = name.to_lowercase();
        if n.contains("head") || n.contains("buds") || n.contains("airpod")
            || n.contains("speaker") || n.contains("audio")
        {
            "audio-card".into()
        } else if n.contains("keyboard") {
            "input-keyboard".into()
        } else if n.contains("mouse") {
            "input-mouse".into()
        } else if n.contains("phone") {
            "phone".into()
        } else {
            String::new()
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn mac_from_dev_form() {
            assert_eq!(
                mac_from_instance(r"BTHENUM\Dev_A4F6E8279B47\7&abc123&0&BluetoothDevice"),
                Some("A4:F6:E8:27:9B:47".into())
            );
        }

        #[test]
        fn mac_from_transport_form_ignores_base_uuid() {
            // The base-UUID tail (00805F9B34FB) sits mid-string; the real MAC is in
            // the last '\'-segment.
            let id = r"BTHENUM\{0000110E-0000-1000-8000-00805F9B34FB}_VID&0001004C_PID&761E\A&3B6AA2F9&0&A4F6E8279B47_C00000000";
            assert_eq!(mac_from_instance(id), Some("A4:F6:E8:27:9B:47".into()));
        }

        #[test]
        fn mac_from_ble_form_ignores_base_uuid() {
            // BLE devices put the MAC after the GUID; the base-UUID inside the
            // braces must not be matched.
            let id = r"BTHLEDEVICE\{00001800-0000-1000-8000-00805F9B34FB}_5ECDFAA32F4C\B&65B0B75&0&0001";
            assert_eq!(mac_from_instance(id), Some("5E:CD:FA:A3:2F:4C".into()));
        }

        #[test]
        fn no_mac_returns_none() {
            assert_eq!(mac_from_instance(r"USB\VID_8087&PID_0029\5&12abc"), None);
        }

        #[test]
        fn subprofile_detection() {
            assert!(is_subprofile("Isra iPhone Transporte AVRCP"));
            assert!(is_subprofile("Headset A2DP"));
            assert!(!is_subprofile("Isra iPhone"));
            assert!(!is_subprofile("DualSense Wireless Controller"));
        }

        #[test]
        fn icon_heuristics() {
            assert_eq!(icon_for("Sony WH-1000 Headphones"), "audio-card");
            assert_eq!(icon_for("Logitech K380 Keyboard"), "input-keyboard");
            assert_eq!(icon_for("MX Master Mouse"), "input-mouse");
            assert_eq!(icon_for("Isra iPhone"), "phone");
            assert_eq!(icon_for("DualSense Wireless Controller"), "");
        }
    }
}

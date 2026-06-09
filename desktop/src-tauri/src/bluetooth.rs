//! Bluetooth device management.
//!
//! Linux: full control via `bluetoothctl`. Windows: list paired devices and
//! toggle the adapter via PnP (connect/disconnect is out of scope — Windows has
//! no simple CLI for it).

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct BtDevice {
    pub mac: String,
    pub name: String,
    pub paired: bool,
    pub connected: bool,
    pub trusted: bool,
    pub icon: String, // audio-card, input-keyboard, phone, ...
}

#[derive(Clone, Debug, Serialize)]
pub struct BtState {
    pub available: bool,
    pub powered: bool,
    pub devices: Vec<BtDevice>,
}

// ── Linux (bluetoothctl) ──────────────────────────────────────────────────────

#[cfg(not(windows))]
pub use unix_impl::{connect, disconnect, is_available, set_power, set_trust, state};

#[cfg(not(windows))]
mod unix_impl {
    use super::{BtDevice, BtState};
    use std::process::Command;

    pub fn is_available() -> bool {
        Command::new("bluetoothctl")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn state() -> BtState {
        if !is_available() {
            return BtState {
                available: false,
                powered: false,
                devices: Vec::new(),
            };
        }
        BtState {
            available: true,
            powered: powered(),
            devices: devices(),
        }
    }

    fn powered() -> bool {
        run(&["show"])
            .map(|s| s.lines().any(|l| l.trim().starts_with("Powered: yes")))
            .unwrap_or(false)
    }

    fn devices() -> Vec<BtDevice> {
        let list = match run(&["devices"]) {
            Some(t) => t,
            None => return Vec::new(),
        };
        let mut out = Vec::new();
        for line in list.lines() {
            // "Device AA:BB:CC:DD:EE:FF Name Here"
            let parts: Vec<&str> = line.trim().splitn(3, ' ').collect();
            if parts.len() >= 2 && parts[0] == "Device" {
                let mac = parts[1].to_string();
                let name = parts.get(2).map(|s| s.to_string()).unwrap_or_else(|| mac.clone());
                out.push(info(&mac, name));
            }
        }
        out
    }

    fn info(mac: &str, fallback_name: String) -> BtDevice {
        let text = run(&["info", mac]).unwrap_or_default();
        let field = |key: &str| -> Option<String> {
            text.lines()
                .find_map(|l| l.trim().strip_prefix(key).map(|v| v.trim().to_string()))
        };
        let name = field("Name:").unwrap_or(fallback_name);
        let yes = |k: &str| field(k).map(|v| v == "yes").unwrap_or(false);
        BtDevice {
            mac: mac.to_string(),
            name,
            paired: yes("Paired:"),
            connected: yes("Connected:"),
            trusted: yes("Trusted:"),
            icon: field("Icon:").unwrap_or_default(),
        }
    }

    pub fn connect(mac: &str) -> Result<(), String> {
        run_cmd(&["connect", mac])
    }

    pub fn disconnect(mac: &str) -> Result<(), String> {
        run_cmd(&["disconnect", mac])
    }

    pub fn set_power(on: bool) -> Result<(), String> {
        run_cmd(&["power", if on { "on" } else { "off" }])
    }

    pub fn set_trust(mac: &str, trust: bool) -> Result<(), String> {
        run_cmd(&[if trust { "trust" } else { "untrust" }, mac])
    }

    fn run(args: &[&str]) -> Option<String> {
        let out = Command::new("bluetoothctl").args(args).output().ok()?;
        if out.status.success() {
            String::from_utf8(out.stdout).ok()
        } else {
            None
        }
    }

    fn run_cmd(args: &[&str]) -> Result<(), String> {
        // bluetoothctl returns success even when the action reports an error in
        // text, so surface stdout/stderr on obvious failures.
        let out = Command::new("bluetoothctl")
            .args(args)
            .output()
            .map_err(|e| format!("bluetoothctl not found: {e}"))?;
        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains("Failed") || stdout.contains("not available") {
            let line = stdout
                .lines()
                .find(|l| l.contains("Failed") || l.contains("not available"))
                .unwrap_or("operation failed");
            return Err(line.trim().to_string());
        }
        if out.status.success() {
            Ok(())
        } else {
            Err(format!("bluetoothctl {} failed", args.join(" ")))
        }
    }
}

// ── Windows (PnP) ─────────────────────────────────────────────────────────────

#[cfg(windows)]
pub use win_impl::{connect, disconnect, is_available, set_power, set_trust, state};

#[cfg(windows)]
mod win_impl {
    use super::{BtDevice, BtState};
    use std::process::Command;

    fn ps(script: &str) -> Option<String> {
        let out = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    }

    /// A paired remote device is enumerated under BTHENUM/BTHLE; anything else of
    /// class Bluetooth (the USB/PCI radio and the MS enumerators) is "infrastructure".
    fn is_paired_device(instance_id: &str) -> bool {
        let id = instance_id.to_ascii_uppercase();
        id.starts_with("BTHENUM") || id.starts_with("BTHLE")
    }

    pub fn is_available() -> bool {
        ps("(Get-PnpDevice -Class Bluetooth -PresentOnly -ErrorAction SilentlyContinue | Measure-Object).Count")
            .and_then(|s| s.trim().parse::<i64>().ok())
            .map(|n| n > 0)
            .unwrap_or(false)
    }

    pub fn state() -> BtState {
        let json = match ps(
            "Get-PnpDevice -Class Bluetooth -PresentOnly -ErrorAction SilentlyContinue | \
             Select-Object InstanceId,FriendlyName,Status | ConvertTo-Json -Compress",
        ) {
            Some(j) if !j.trim().is_empty() => j,
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
    pub fn set_power(on: bool) -> Result<(), String> {
        let verb = if on { "Enable-PnpDevice" } else { "Disable-PnpDevice" };
        let action = format!(
            "Get-PnpDevice -Class Bluetooth | Where-Object {{ $_.InstanceId -notlike 'BTHENUM*' \
             -and $_.InstanceId -notlike 'BTHLE*' }} | {verb} -Confirm:$false"
        );
        crate::winutil::run_elevated(&action)
    }

    pub fn connect(_mac: &str) -> Result<(), String> {
        Err("Connecting Bluetooth devices isn't supported on Windows yet — pair/connect from \
             Windows Settings. You can still toggle the adapter here."
            .into())
    }

    pub fn disconnect(_mac: &str) -> Result<(), String> {
        Err("Disconnecting Bluetooth devices isn't supported on Windows yet — use Windows Settings.".into())
    }

    pub fn set_trust(_mac: &str, _trust: bool) -> Result<(), String> {
        Err("Trust management isn't applicable on Windows.".into())
    }

    /// Extract a 12-hex MAC from a Bluetooth PnP instance id.
    /// Handles both `...\DEV_AABBCCDDEEFF\...` (paired device) and the profile
    /// transport form `...\<seg>&AABBCCDDEEFF_C00000000` (AVRCP/A2DP sub-nodes).
    pub(crate) fn mac_from_instance(instance: &str) -> Option<String> {
        let upper = instance.to_ascii_uppercase();
        // Format A: the MAC follows "DEV_".
        if let Some(p) = upper.find("DEV_") {
            if let Some(mac) = take_mac(&upper[p + 4..]) {
                return Some(mac);
            }
        }
        // Format B: scan only the last '\'-segment for a 12-hex run (avoids the
        // 12-hex tail of the Bluetooth base-UUID that appears mid-string).
        let last = upper.rsplit('\\').next().unwrap_or(&upper);
        let mut run = String::new();
        for c in last.chars() {
            if c.is_ascii_hexdigit() {
                run.push(c);
                if run.len() == 12 {
                    return Some(fmt_mac(&run));
                }
            } else {
                run.clear();
            }
        }
        None
    }

    /// Read up to 12 leading hex digits and format them as a MAC.
    fn take_mac(s: &str) -> Option<String> {
        let hex: String = s.chars().take_while(|c| c.is_ascii_hexdigit()).take(12).collect();
        (hex.len() == 12).then(|| fmt_mac(&hex))
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

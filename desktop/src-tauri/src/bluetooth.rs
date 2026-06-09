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

        let mut devices = Vec::new();
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
                devices.push(BtDevice {
                    mac: mac_from_instance(&instance).unwrap_or(instance),
                    name: name.clone(),
                    paired: true,
                    connected: ok,
                    trusted: false,
                    icon: icon_for(&name),
                });
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

    /// Extract the 12-hex MAC from a "...Dev_AABBCCDDEEFF..." instance id.
    fn mac_from_instance(instance: &str) -> Option<String> {
        let upper = instance.to_ascii_uppercase();
        let idx = upper.find("DEV_")? + 4;
        let hex: String = upper[idx..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .collect();
        if hex.len() < 12 {
            return None;
        }
        let bytes: Vec<String> = hex[..12]
            .as_bytes()
            .chunks(2)
            .map(|c| String::from_utf8_lossy(c).to_string())
            .collect();
        Some(bytes.join(":"))
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
}

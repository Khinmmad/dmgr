//! Windows backend via PowerShell (Get-PnpDevice / Enable-PnpDevice / ...).
//!
//! NOTE: written but UNVERIFIED — it is `#[cfg(target_os = "windows")]` and the
//! project is developed on Arch Linux, so it has not been compiled or run on
//! Windows yet. Uses PowerShell instead of raw SetupAPI to avoid a heavy unsafe
//! dependency for a first cut. Audio/Bluetooth/kernel-module panels are Linux-only
//! and degrade to empty on Windows.

use super::DeviceBackend;
use dmgr_core::device::{Bus, Device, DeviceStatus};
use std::collections::HashMap;
use std::process::Command;

pub struct WindowsBackend;

impl DeviceBackend for WindowsBackend {
    fn scan(&self) -> Result<Vec<Device>, String> {
        let json = powershell(
            "Get-PnpDevice | Select-Object InstanceId,FriendlyName,Class,Status,Present,Service,Manufacturer | ConvertTo-Json -Compress",
        )?;
        let value: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| format!("parse Get-PnpDevice: {e}"))?;

        let items = match value {
            serde_json::Value::Array(a) => a,
            obj @ serde_json::Value::Object(_) => vec![obj], // single device
            _ => Vec::new(),
        };

        let mut devices = Vec::new();
        for it in items {
            let get = |k: &str| it.get(k).and_then(|v| v.as_str()).map(str::to_string);
            let instance = match get("InstanceId") {
                Some(s) if !s.is_empty() => s,
                _ => continue,
            };
            let class = get("Class").unwrap_or_default();
            let status = get("Status").unwrap_or_default();
            let present = it.get("Present").and_then(|v| v.as_bool()).unwrap_or(true);
            let name = get("FriendlyName").unwrap_or_else(|| instance.clone());

            let bus = class_to_bus(&class);
            let mut dev = Device::new(instance.clone(), name, bus, class.clone(), instance.clone());
            dev.driver = get("Service");
            dev.vendor = get("Manufacturer");
            dev.status = status_map(&status, present);
            dev.authorized = status.eq_ignore_ascii_case("OK");
            dev.properties = HashMap::new();
            devices.push(dev);
        }
        Ok(devices)
    }

    fn available_drivers(&self, _path: &str) -> Result<Vec<String>, String> {
        // Windows has no per-device "bind a different driver" like Linux. Driver
        // packages are managed by pnputil/Windows Update, not bound ad-hoc, so we
        // don't populate the bind dropdown (which would be a dead end). The current
        // driver package and version are surfaced read-only via `advanced_details`.
        Ok(Vec::new())
    }

    fn get_property(&self, path: &str, property: &str) -> Result<Option<String>, String> {
        let script = format!(
            "(Get-PnpDeviceProperty -InstanceId '{}' -KeyName '{}').Data",
            escape(path),
            escape(property)
        );
        powershell(&script).map(|s| {
            let s = s.trim().to_string();
            (!s.is_empty()).then_some(s)
        })
    }

    fn set_property(&self, _: &str, _: &str, _: &str) -> Result<(), String> {
        Err("Editing device properties is not supported on Windows".into())
    }

    fn bind(&self, _: &str, _: &str) -> Result<(), String> {
        Err("Driver bind is not supported on Windows (use Enable/Disable)".into())
    }

    fn unbind(&self, _: &str) -> Result<(), String> {
        Err("Driver unbind is not supported on Windows (use Enable/Disable)".into())
    }

    fn set_enabled(&self, path: &str, enabled: bool) -> Result<(), String> {
        let verb = if enabled { "Enable-PnpDevice" } else { "Disable-PnpDevice" };
        let action = format!("{verb} -InstanceId '{}' -Confirm:$false", escape(path));
        // Enable/Disable-PnpDevice requires Administrator. Run elevated via UAC:
        // `Start-Process -Verb RunAs` prompts if we're not already elevated, and
        // runs silently (no prompt) if we are.
        crate::winutil::run_elevated(&action)
    }
}

fn powershell(script: &str) -> Result<String, String> {
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .map_err(|e| format!("powershell failed: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

/// Minimal quoting safety for values interpolated into single-quoted PS strings.
fn escape(s: &str) -> String {
    s.replace('\'', "''")
}

fn class_to_bus(class: &str) -> Bus {
    match class.to_ascii_uppercase().as_str() {
        "USB" => Bus::Usb,
        "DISPLAY" | "MONITOR" => Bus::Drm,
        "NET" => Bus::Net,
        "MEDIA" | "AUDIOENDPOINT" | "SOUND" => Bus::Audio,
        "HIDCLASS" | "HID" => Bus::Hid,
        "KEYBOARD" | "MOUSE" => Bus::Input,
        "DISKDRIVE" | "VOLUME" | "CDROM" => Bus::Block,
        "PORTS" => Bus::Tty,
        "" => Bus::Unknown("Other".into()),
        other => Bus::Unknown(capitalize(other)),
    }
}

fn capitalize(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let mut c = lower.chars();
    match c.next() {
        Some(f) => f.to_ascii_uppercase().to_string() + c.as_str(),
        None => lower,
    }
}

fn status_map(status: &str, present: bool) -> DeviceStatus {
    if !present {
        return DeviceStatus::Offline;
    }
    match status.to_ascii_uppercase().as_str() {
        "OK" => DeviceStatus::Online,
        "ERROR" => DeviceStatus::Error,
        "DEGRADED" => DeviceStatus::Suspended,
        _ => DeviceStatus::Unknown,
    }
}

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
        run_elevated(&action)
    }
}

/// Run a PowerShell command elevated (UAC). Returns Ok only if the elevated
/// child exits 0. Distinguishes a cancelled UAC prompt for a friendlier message.
fn run_elevated(action: &str) -> Result<(), String> {
    // Pass the inner command as a base64 -EncodedCommand to sidestep all nested
    // quoting through Start-Process -ArgumentList.
    let encoded = encode_command(action);
    let launcher = format!(
        "try {{ $p = Start-Process powershell -Verb RunAs -Wait -PassThru -WindowStyle Hidden \
         -ArgumentList '-NoProfile','-NonInteractive','-EncodedCommand','{encoded}'; \
         exit $p.ExitCode }} catch {{ exit 1223 }}"
    );
    let out = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &launcher])
        .output()
        .map_err(|e| format!("failed to launch elevated PowerShell: {e}"))?;
    match out.status.code() {
        Some(0) => Ok(()),
        Some(1223) => Err("Elevation cancelled — approve the Administrator (UAC) prompt to continue".into()),
        Some(c) => Err(format!("Action failed with administrator rights (exit code {c})")),
        None => Err("Elevated action terminated unexpectedly".into()),
    }
}

/// Encode a PowerShell command for `-EncodedCommand` (base64 of UTF-16LE).
fn encode_command(ps: &str) -> String {
    let utf16le: Vec<u8> = ps.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    base64(&utf16le)
}

/// Minimal standard-alphabet base64 (no external crate needed).
fn base64(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((n >> 18) & 63) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((n >> 6) & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
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

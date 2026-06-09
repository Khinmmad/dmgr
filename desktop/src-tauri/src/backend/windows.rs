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
        // Prefer the fast native SetupAPI enumeration; fall back to PowerShell
        // (Get-PnpDevice) only if it yields nothing, so behavior never regresses.
        match scan_native() {
            Ok(devs) if !devs.is_empty() => Ok(devs),
            _ => scan_powershell(),
        }
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

// ── Native enumeration (SetupAPI) ─────────────────────────────────────────────

/// Fast device scan via SetupDi* — no PowerShell process per scan.
fn scan_native() -> Result<Vec<Device>, String> {
    use windows::core::PCWSTR;
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsW,
        DIGCF_ALLCLASSES, DIGCF_PRESENT, SP_DEVINFO_DATA,
    };
    use windows::Win32::Devices::Properties::{
        DEVPKEY_Device_Class, DEVPKEY_Device_DeviceDesc, DEVPKEY_Device_DevNodeStatus,
        DEVPKEY_Device_FriendlyName, DEVPKEY_Device_Manufacturer, DEVPKEY_Device_ProblemCode,
        DEVPKEY_Device_Service,
    };
    use windows::Win32::Foundation::HWND;

    unsafe {
        let hdev = SetupDiGetClassDevsW(None, PCWSTR::null(), HWND::default(), DIGCF_PRESENT | DIGCF_ALLCLASSES)
            .map_err(|e| e.message().to_string())?;

        let mut devices = Vec::new();
        let mut data = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };

        let mut i = 0u32;
        while SetupDiEnumDeviceInfo(hdev, i, &mut data).is_ok() {
            i += 1;

            let instance = match instance_id(hdev, &data) {
                Some(s) if !s.is_empty() => s,
                _ => continue,
            };
            let class = string_prop(hdev, &data, &DEVPKEY_Device_Class).unwrap_or_default();
            let name = string_prop(hdev, &data, &DEVPKEY_Device_FriendlyName)
                .or_else(|| string_prop(hdev, &data, &DEVPKEY_Device_DeviceDesc))
                .unwrap_or_else(|| instance.clone());
            let problem = u32_prop(hdev, &data, &DEVPKEY_Device_ProblemCode).unwrap_or(0);
            let _status = u32_prop(hdev, &data, &DEVPKEY_Device_DevNodeStatus);

            let bus = class_to_bus(&class);
            let mut dev = Device::new(instance.clone(), name, bus, class, instance);
            dev.driver = string_prop(hdev, &data, &DEVPKEY_Device_Service);
            dev.vendor = string_prop(hdev, &data, &DEVPKEY_Device_Manufacturer);
            dev.status = problem_to_status(problem);
            dev.authorized = problem != 22; // CM_PROB_DISABLED
            dev.properties = HashMap::new();
            devices.push(dev);
        }

        let _ = SetupDiDestroyDeviceInfoList(hdev);
        Ok(devices)
    }
}

/// CM_PROB_* problem code → status. 0 = working, 22 = disabled, else error.
fn problem_to_status(problem: u32) -> DeviceStatus {
    match problem {
        0 => DeviceStatus::Online,
        22 => DeviceStatus::Offline,
        _ => DeviceStatus::Error,
    }
}

unsafe fn instance_id(
    hdev: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    data: &windows::Win32::Devices::DeviceAndDriverInstallation::SP_DEVINFO_DATA,
) -> Option<String> {
    use windows::Win32::Devices::DeviceAndDriverInstallation::SetupDiGetDeviceInstanceIdW;
    let mut size = 0u32;
    let _ = SetupDiGetDeviceInstanceIdW(hdev, data, None, Some(&mut size));
    if size == 0 {
        return None;
    }
    let mut buf = vec![0u16; size as usize];
    SetupDiGetDeviceInstanceIdW(hdev, data, Some(&mut buf), Some(&mut size)).ok()?;
    Some(String::from_utf16_lossy(&buf).trim_end_matches('\0').to_string())
}

unsafe fn string_prop(
    hdev: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    data: &windows::Win32::Devices::DeviceAndDriverInstallation::SP_DEVINFO_DATA,
    key: &windows::Win32::Devices::Properties::DEVPROPKEY,
) -> Option<String> {
    use windows::Win32::Devices::DeviceAndDriverInstallation::SetupDiGetDevicePropertyW;
    use windows::Win32::Devices::Properties::{DEVPROPTYPE, DEVPROP_TYPE_STRING};

    let mut ptype = DEVPROPTYPE(0);
    let mut size = 0u32;
    let _ = SetupDiGetDevicePropertyW(hdev, data, key, &mut ptype, None, Some(&mut size), 0);
    if size == 0 {
        return None;
    }
    let mut buf = vec![0u8; size as usize];
    SetupDiGetDevicePropertyW(hdev, data, key, &mut ptype, Some(&mut buf), Some(&mut size), 0).ok()?;
    if ptype != DEVPROP_TYPE_STRING {
        return None;
    }
    let u16s = std::slice::from_raw_parts(buf.as_ptr() as *const u16, buf.len() / 2);
    let s = String::from_utf16_lossy(u16s);
    let s = s.trim_end_matches('\0').trim().to_string();
    (!s.is_empty()).then_some(s)
}

unsafe fn u32_prop(
    hdev: windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO,
    data: &windows::Win32::Devices::DeviceAndDriverInstallation::SP_DEVINFO_DATA,
    key: &windows::Win32::Devices::Properties::DEVPROPKEY,
) -> Option<u32> {
    use windows::Win32::Devices::DeviceAndDriverInstallation::SetupDiGetDevicePropertyW;
    use windows::Win32::Devices::Properties::DEVPROPTYPE;

    let mut ptype = DEVPROPTYPE(0);
    let mut size = 0u32;
    let _ = SetupDiGetDevicePropertyW(hdev, data, key, &mut ptype, None, Some(&mut size), 0);
    if size as usize != 4 {
        return None;
    }
    let mut buf = [0u8; 4];
    SetupDiGetDevicePropertyW(hdev, data, key, &mut ptype, Some(&mut buf), Some(&mut size), 0).ok()?;
    Some(u32::from_ne_bytes(buf))
}

/// Fallback scan via PowerShell (Get-PnpDevice).
fn scan_powershell() -> Result<Vec<Device>, String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_to_bus_maps_known_and_unknown() {
        assert_eq!(class_to_bus("USB"), Bus::Usb);
        assert_eq!(class_to_bus("net"), Bus::Net);
        assert_eq!(class_to_bus("Display"), Bus::Drm);
        assert_eq!(class_to_bus("Media"), Bus::Audio);
        assert_eq!(class_to_bus("Keyboard"), Bus::Input);
        assert_eq!(class_to_bus(""), Bus::Unknown("Other".into()));
        assert_eq!(class_to_bus("Printer"), Bus::Unknown("Printer".into()));
    }

    #[test]
    fn status_map_handles_presence_and_state() {
        assert_eq!(status_map("OK", true), DeviceStatus::Online);
        assert_eq!(status_map("Error", true), DeviceStatus::Error);
        assert_eq!(status_map("Degraded", true), DeviceStatus::Suspended);
        assert_eq!(status_map("Whatever", true), DeviceStatus::Unknown);
        // Absent devices are offline regardless of status text.
        assert_eq!(status_map("OK", false), DeviceStatus::Offline);
    }

    #[test]
    fn capitalize_first_letter() {
        assert_eq!(capitalize("printer"), "Printer");
        assert_eq!(capitalize("HID"), "Hid");
        assert_eq!(capitalize(""), "");
    }
}

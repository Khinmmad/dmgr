//! macOS backend (scaffold) via `system_profiler -json`.
//!
//! Lists USB / audio / network / storage / display devices. Read-only: macOS has
//! no per-device enable/disable or driver bind like Linux/Windows, so the write
//! operations return informative errors.
//!
//! NOTE: written and type-checked for the macOS target, but not yet runtime-tested
//! on a Mac. Property reads and the exact `system_profiler` JSON shape may need
//! tweaking against real output.

use super::DeviceBackend;
use dmgr_core::device::{Bus, Device, DeviceStatus};
use std::process::Command;

pub struct MacosBackend;

impl DeviceBackend for MacosBackend {
    fn scan(&self) -> Result<Vec<Device>, String> {
        let mut devices = Vec::new();
        let sources = [
            ("SPUSBDataType", Bus::Usb),
            ("SPAudioDataType", Bus::Audio),
            ("SPNetworkDataType", Bus::Net),
            ("SPStorageDataType", Bus::Block),
            ("SPDisplaysDataType", Bus::Drm),
        ];
        for (data_type, bus) in sources {
            collect(data_type, &bus, &mut devices);
        }
        Ok(devices)
    }

    fn available_drivers(&self, _path: &str) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }

    fn get_property(&self, _path: &str, _property: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn set_property(&self, _: &str, _: &str, _: &str) -> Result<(), String> {
        Err("Editing device properties is not supported on macOS".into())
    }

    fn bind(&self, _: &str, _: &str) -> Result<(), String> {
        Err("Driver bind is not supported on macOS".into())
    }

    fn unbind(&self, _: &str) -> Result<(), String> {
        Err("Driver unbind is not supported on macOS".into())
    }

    fn set_enabled(&self, _: &str, _: bool) -> Result<(), String> {
        Err("Enable/disable is not supported on macOS".into())
    }
}

/// Run `system_profiler -json <data_type>` and walk its tree into devices.
fn collect(data_type: &str, bus: &Bus, out: &mut Vec<Device>) {
    let json = match run_sp(data_type) {
        Some(j) => j,
        None => return,
    };
    let value: serde_json::Value = match serde_json::from_str(&json) {
        Ok(v) => v,
        Err(_) => return,
    };
    if let Some(arr) = value.get(data_type).and_then(|x| x.as_array()) {
        let mut idx = 0usize;
        for item in arr {
            walk(item, bus, data_type, &mut idx, out);
        }
    }
}

/// Recursively turn `_name`-bearing nodes (and their `_items`) into devices.
fn walk(item: &serde_json::Value, bus: &Bus, prefix: &str, idx: &mut usize, out: &mut Vec<Device>) {
    if let Some(name) = item.get("_name").and_then(|n| n.as_str()) {
        let id = format!("{prefix}-{idx}");
        *idx += 1;

        let get = |k: &str| item.get(k).and_then(|v| v.as_str()).map(str::to_string);
        let mut dev = Device::new(id.clone(), name.to_string(), bus.clone(), prefix.to_string(), id);
        dev.vendor = get("manufacturer").or_else(|| get("vendor_id"));
        dev.model = get("device_model").or_else(|| get("_name"));
        dev.vendor_id = get("vendor_id");
        dev.model_id = get("product_id");
        dev.status = DeviceStatus::Online;
        out.push(dev);
    }

    if let Some(children) = item.get("_items").and_then(|x| x.as_array()) {
        for child in children {
            walk(child, bus, prefix, idx, out);
        }
    }
}

fn run_sp(data_type: &str) -> Option<String> {
    let out = Command::new("system_profiler")
        .args(["-json", data_type])
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

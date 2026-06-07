//! Bluetooth device management via `bluetoothctl`. Degrades gracefully when absent.

use serde::Serialize;
use std::process::Command;

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

// ── internals ───────────────────────────────────────────────────────────────

fn run(args: &[&str]) -> Option<String> {
    let out = Command::new("bluetoothctl").args(args).output().ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout).ok()
    } else {
        None
    }
}

fn run_cmd(args: &[&str]) -> Result<(), String> {
    // bluetoothctl returns success even when the action reports an error in text,
    // so surface stdout/stderr on obvious failures.
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

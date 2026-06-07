//! Tauri command surface — everything the React frontend can invoke.

use crate::{audio, bluetooth, privileged};
use dmgr_core::{device::Device, properties, sysfs};
use serde::Serialize;

#[derive(Serialize)]
pub struct Capabilities {
    pub audio: bool,
    pub bluetooth: bool,
    pub root: bool,
}

// ── Devices ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn scan_devices() -> Result<Vec<Device>, String> {
    sysfs::scan_all_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_available_drivers(path: String) -> Result<Vec<String>, String> {
    properties::get_available_drivers(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_property(path: String, property: String) -> Result<Option<String>, String> {
    properties::get_property(&path, &property).map_err(|e| e.to_string())
}

/// Modify an editable sysfs property (privileged).
#[tauri::command]
pub fn set_property(path: String, property: String, value: String) -> Result<(), String> {
    privileged::run_privileged(&["set", &path, &property, &value])
}

#[tauri::command]
pub fn bind_driver(path: String, driver: String) -> Result<(), String> {
    privileged::run_privileged(&["bind", &path, &driver])
}

#[tauri::command]
pub fn unbind_driver(path: String) -> Result<(), String> {
    privileged::run_privileged(&["unbind", &path])
}

/// Windows-style "Enable/Disable device" — toggles the USB `authorized` flag.
#[tauri::command]
pub fn set_device_enabled(path: String, enabled: bool) -> Result<(), String> {
    let value = if enabled { "1" } else { "0" };
    privileged::run_privileged(&["set", &path, "authorized", value])
}

// ── Audio ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn audio_outputs() -> Vec<audio::AudioDevice> {
    audio::list_sinks()
}

#[tauri::command]
pub fn audio_inputs() -> Vec<audio::AudioDevice> {
    audio::list_sources()
}

#[tauri::command]
pub fn audio_set_default_output(name: String) -> Result<(), String> {
    audio::set_default_sink(&name)
}

#[tauri::command]
pub fn audio_set_default_input(name: String) -> Result<(), String> {
    audio::set_default_source(&name)
}

#[tauri::command]
pub fn audio_set_volume(name: String, percent: u32) -> Result<(), String> {
    audio::set_sink_volume(&name, percent)
}

#[tauri::command]
pub fn audio_set_mute(name: String, muted: bool) -> Result<(), String> {
    audio::set_sink_mute(&name, muted)
}

// ── Bluetooth ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn bt_state() -> bluetooth::BtState {
    bluetooth::state()
}

#[tauri::command]
pub fn bt_connect(mac: String) -> Result<(), String> {
    bluetooth::connect(&mac)
}

#[tauri::command]
pub fn bt_disconnect(mac: String) -> Result<(), String> {
    bluetooth::disconnect(&mac)
}

#[tauri::command]
pub fn bt_set_power(on: bool) -> Result<(), String> {
    bluetooth::set_power(on)
}

#[tauri::command]
pub fn bt_set_trust(mac: String, trust: bool) -> Result<(), String> {
    bluetooth::set_trust(&mac, trust)
}

// ── Meta ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn capabilities() -> Capabilities {
    Capabilities {
        audio: audio::is_available(),
        bluetooth: bluetooth::is_available(),
        root: std::env::var("USER").map(|u| u == "root").unwrap_or(false),
    }
}

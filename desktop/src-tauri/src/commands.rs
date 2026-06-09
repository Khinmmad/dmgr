//! Tauri command surface — everything the React frontend can invoke.

use crate::backend::Backend;
use crate::{audio, bluetooth, details, kernel, platform};
use dmgr_core::device::Device;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct Capabilities {
    pub audio: bool,
    pub audio_backend: String,
    pub bluetooth: bool,
    pub root: bool,
}

fn audio_or_err() -> Result<Box<dyn audio::AudioBackend>, String> {
    audio::detect().ok_or_else(|| "no audio backend (pactl/wpctl/alsa) available".to_string())
}

// ── Devices (routed through the OS-abstracted backend) ───────────────────────

#[tauri::command]
pub fn scan_devices(backend: State<'_, Backend>) -> Result<Vec<Device>, String> {
    backend.scan()
}

#[tauri::command]
pub fn get_available_drivers(
    backend: State<'_, Backend>,
    path: String,
) -> Result<Vec<String>, String> {
    backend.available_drivers(&path)
}

#[tauri::command]
pub fn get_property(
    backend: State<'_, Backend>,
    path: String,
    property: String,
) -> Result<Option<String>, String> {
    backend.get_property(&path, &property)
}

/// Modify an editable property (privileged).
#[tauri::command]
pub fn set_property(
    backend: State<'_, Backend>,
    path: String,
    property: String,
    value: String,
) -> Result<(), String> {
    backend.set_property(&path, &property, &value)
}

#[tauri::command]
pub fn bind_driver(
    backend: State<'_, Backend>,
    path: String,
    driver: String,
) -> Result<(), String> {
    backend.bind(&path, &driver)
}

#[tauri::command]
pub fn unbind_driver(backend: State<'_, Backend>, path: String) -> Result<(), String> {
    backend.unbind(&path)
}

/// Windows-style "Enable/Disable device".
#[tauri::command]
pub fn set_device_enabled(
    backend: State<'_, Backend>,
    path: String,
    enabled: bool,
) -> Result<(), String> {
    backend.set_enabled(&path, enabled)
}

// ── Audio ────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn audio_outputs() -> Vec<audio::AudioDevice> {
    audio::detect().map(|b| b.outputs()).unwrap_or_default()
}

#[tauri::command]
pub fn audio_inputs() -> Vec<audio::AudioDevice> {
    audio::detect().map(|b| b.inputs()).unwrap_or_default()
}

#[tauri::command]
pub fn audio_set_default_output(name: String) -> Result<(), String> {
    audio_or_err()?.set_default_output(&name)
}

#[tauri::command]
pub fn audio_set_default_input(name: String) -> Result<(), String> {
    audio_or_err()?.set_default_input(&name)
}

#[tauri::command]
pub fn audio_set_volume(name: String, percent: u32) -> Result<(), String> {
    audio_or_err()?.set_volume(&name, percent)
}

#[tauri::command]
pub fn audio_set_mute(name: String, muted: bool) -> Result<(), String> {
    audio_or_err()?.set_mute(&name, muted)
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
        audio_backend: audio::backend_name().to_string(),
        bluetooth: bluetooth::is_available(),
        root: is_privileged(),
    }
}

/// Whether we're running with elevated rights (root on Unix, Administrator on Windows).
#[cfg(windows)]
fn is_privileged() -> bool {
    crate::privileged::can_elevate()
}

#[cfg(not(windows))]
fn is_privileged() -> bool {
    std::env::var("USER").map(|u| u == "root").unwrap_or(false)
}

#[tauri::command]
pub fn platform_info() -> platform::Platform {
    platform::detect()
}

// ── Advanced details ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn advanced_details(path: String, bus: String) -> Vec<details::DetailItem> {
    details::advanced(&path, &bus)
}

// ── Kernel modules ───────────────────────────────────────────────────────────

#[tauri::command]
pub fn kernel_modules() -> Vec<kernel::KernelModule> {
    kernel::list()
}

#[tauri::command]
pub fn kernel_module_info(name: String) -> kernel::ModuleInfo {
    kernel::info(&name)
}

#[tauri::command]
pub fn kernel_module_load(name: String) -> Result<(), String> {
    kernel::load(&name)
}

#[tauri::command]
pub fn kernel_module_unload(name: String) -> Result<(), String> {
    kernel::unload(&name)
}

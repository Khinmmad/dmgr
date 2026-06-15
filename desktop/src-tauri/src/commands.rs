//! Tauri command surface — everything the React frontend can invoke.

use crate::backend::Backend;
use crate::{audio, bluetooth, details, kernel, platform, system};
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

/// Reload the kernel module behind a device's driver (`modprobe -r` + `modprobe`)
/// — the Linux analog of "update driver". Fails harmlessly if the module is in
/// use. `driver` is validated to a module-name charset to keep the shell safe.
#[tauri::command]
pub fn reload_driver(driver: String) -> Result<(), String> {
    if driver.is_empty()
        || !driver.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("invalid module name".into());
    }
    #[cfg(not(windows))]
    {
        crate::privileged::run_pkexec(
            "sh",
            &["-c", &format!("modprobe -r {driver} && modprobe {driver}")],
        )
    }
    #[cfg(windows)]
    {
        Err("Reloading drivers isn't supported on Windows — use Device Manager.".into())
    }
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

#[tauri::command]
pub fn audio_app_streams() -> Vec<audio::AudioApp> {
    audio::app_streams()
}

#[tauri::command]
pub fn audio_set_app_volume(index: u32, percent: u32) -> Result<(), String> {
    audio::set_app_volume(index, percent)
}

#[tauri::command]
pub fn audio_set_app_mute(index: u32, muted: bool) -> Result<(), String> {
    audio::set_app_mute(index, muted)
}

// ── Bluetooth ────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn bt_state() -> bluetooth::BtState {
    bluetooth::state().await
}

#[tauri::command]
pub async fn bt_pair(mac: String) -> Result<(), bluetooth::BtError> {
    bluetooth::pair(&mac).await
}

#[tauri::command]
pub async fn bt_connect(mac: String) -> Result<(), bluetooth::BtError> {
    bluetooth::connect(&mac).await
}

#[tauri::command]
pub async fn bt_disconnect(mac: String) -> Result<(), bluetooth::BtError> {
    bluetooth::disconnect(&mac).await
}

#[tauri::command]
pub async fn bt_set_power(on: bool) -> Result<(), bluetooth::BtError> {
    bluetooth::set_power(on).await
}

#[tauri::command]
pub async fn bt_set_trust(mac: String, trust: bool) -> Result<(), bluetooth::BtError> {
    bluetooth::set_trust(&mac, trust).await
}

#[tauri::command]
pub async fn bt_remove(mac: String) -> Result<(), bluetooth::BtError> {
    bluetooth::remove(&mac).await
}

#[tauri::command]
pub async fn bt_scan(secs: Option<u64>) -> Result<Vec<bluetooth::BtDevice>, bluetooth::BtError> {
    bluetooth::scan(secs.unwrap_or(10)).await
}

// ── Meta ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn capabilities() -> Capabilities {
    Capabilities {
        audio: audio::is_available(),
        audio_backend: audio::backend_name().to_string(),
        bluetooth: bluetooth::is_available().await,
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
    // euid==0 is the truthful check; the `USER` env var can be stale or unset
    // when launched from a desktop entry, sudo, or pkexec.
    crate::privileged::is_root()
}

#[tauri::command]
pub fn platform_info() -> platform::Platform {
    platform::detect()
}

#[tauri::command]
pub fn system_info() -> system::SystemInfo {
    system::info()
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

// ── Windows system shortcuts ──────────────────────────────────────────────────
// Driver replacement and classic-Bluetooth connect aren't safely scriptable on
// Windows, so we hand off to the OS tools that own those flows.

/// Open the Windows Device Manager (devmgmt.msc).
#[tauri::command]
pub fn open_device_manager() -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", "devmgmt.msc"])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("could not open Device Manager: {e}"))
    }
    #[cfg(not(windows))]
    {
        Err("Device Manager is Windows-only".into())
    }
}

/// Open the Windows Bluetooth settings page.
#[tauri::command]
pub fn open_bluetooth_settings() -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", "ms-settings:bluetooth"])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("could not open Bluetooth settings: {e}"))
    }
    #[cfg(not(windows))]
    {
        Err("Windows-only".into())
    }
}

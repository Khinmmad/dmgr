//! OS-abstracted device backend. The frontend/command layer only talks to this
//! trait, so a `WindowsBackend` (SetupAPI/WMI) can be added later without touching
//! the UI. The Linux implementation reuses `dmgr-core` for reads and the privileged
//! helper (`pkexec dmgr-polkit-helper`) for writes.

use dmgr_core::device::Device;

#[cfg(target_os = "linux")]
mod linux;

/// Everything the UI can ask of the host OS regarding devices.
pub trait DeviceBackend: Send + Sync {
    fn scan(&self) -> Result<Vec<Device>, String>;
    fn available_drivers(&self, path: &str) -> Result<Vec<String>, String>;
    fn get_property(&self, path: &str, property: &str) -> Result<Option<String>, String>;
    fn set_property(&self, path: &str, property: &str, value: &str) -> Result<(), String>;
    fn bind(&self, path: &str, driver: &str) -> Result<(), String>;
    fn unbind(&self, path: &str) -> Result<(), String>;
    /// Windows-style enable/disable (Linux: the kernel `authorized` flag).
    fn set_enabled(&self, path: &str, enabled: bool) -> Result<(), String>;
}

/// Managed Tauri state type.
pub type Backend = Box<dyn DeviceBackend>;

/// Build the backend for the current platform.
pub fn current_backend() -> Backend {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxBackend)
    }
    #[cfg(not(target_os = "linux"))]
    {
        // Sprint 5 will add a WindowsBackend here.
        Box::new(Unsupported)
    }
}

#[cfg(not(target_os = "linux"))]
struct Unsupported;

#[cfg(not(target_os = "linux"))]
impl DeviceBackend for Unsupported {
    fn scan(&self) -> Result<Vec<Device>, String> {
        Err("device management is not implemented for this OS yet".into())
    }
    fn available_drivers(&self, _: &str) -> Result<Vec<String>, String> {
        Ok(Vec::new())
    }
    fn get_property(&self, _: &str, _: &str) -> Result<Option<String>, String> {
        Ok(None)
    }
    fn set_property(&self, _: &str, _: &str, _: &str) -> Result<(), String> {
        Err("unsupported OS".into())
    }
    fn bind(&self, _: &str, _: &str) -> Result<(), String> {
        Err("unsupported OS".into())
    }
    fn unbind(&self, _: &str) -> Result<(), String> {
        Err("unsupported OS".into())
    }
    fn set_enabled(&self, _: &str, _: bool) -> Result<(), String> {
        Err("unsupported OS".into())
    }
}

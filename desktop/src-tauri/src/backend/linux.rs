//! Linux backend: reads via `dmgr-core` (sysfs/udev), writes via the privileged helper.

use super::DeviceBackend;
use crate::privileged;
use dmgr_core::{device::Device, properties, sysfs};

pub struct LinuxBackend;

impl DeviceBackend for LinuxBackend {
    fn scan(&self) -> Result<Vec<Device>, String> {
        sysfs::scan_all_devices().map_err(|e| e.to_string())
    }

    fn available_drivers(&self, path: &str) -> Result<Vec<String>, String> {
        properties::get_available_drivers(path).map_err(|e| e.to_string())
    }

    fn get_property(&self, path: &str, property: &str) -> Result<Option<String>, String> {
        properties::get_property(path, property).map_err(|e| e.to_string())
    }

    fn set_property(&self, path: &str, property: &str, value: &str) -> Result<(), String> {
        privileged::run_privileged(&["set", path, property, value])
    }

    fn bind(&self, path: &str, driver: &str) -> Result<(), String> {
        privileged::run_privileged(&["bind", path, driver])
    }

    fn unbind(&self, path: &str) -> Result<(), String> {
        privileged::run_privileged(&["unbind", path])
    }

    fn set_enabled(&self, path: &str, enabled: bool) -> Result<(), String> {
        let value = if enabled { "1" } else { "0" };
        privileged::run_privileged(&["set", path, "authorized", value])
    }
}

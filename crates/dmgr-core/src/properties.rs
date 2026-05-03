use crate::error::Result;
use crate::sysfs;
use log::info;

pub fn get_property(device_sysfs_path: &str, property: &str) -> Result<Option<String>> {
    Ok(sysfs::read_sysfs_file(device_sysfs_path, property))
}

pub fn set_property(device_sysfs_path: &str, property: &str, value: &str) -> Result<()> {
    info!(
        "Setting property {} = {} on {}",
        property, value, device_sysfs_path
    );
    sysfs::write_sysfs_file(device_sysfs_path, property, value)
}

pub fn get_available_drivers(device_sysfs_path: &str) -> Result<Vec<String>> {
    let subsystem = guess_subsystem(device_sysfs_path)?;
    let drivers_path = format!("/sys/bus/{}/drivers", subsystem);

    let mut drivers = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&drivers_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.is_empty() {
                drivers.push(name);
            }
        }
    }

    drivers.sort();
    Ok(drivers)
}

pub fn get_modalias(device_sysfs_path: &str) -> Result<Option<String>> {
    Ok(sysfs::read_sysfs_file(device_sysfs_path, "modalias"))
}

fn guess_subsystem(device_path: &str) -> Result<String> {
    let path = std::path::Path::new(device_path);

    let subsystem_link = path.join("subsystem");
    if let Ok(target) = std::fs::read_link(&subsystem_link) {
        if let Some(name) = target.file_name() {
            return Ok(name.to_string_lossy().to_string());
        }
    }

    for segment in path.ancestors() {
        let segment_str = segment.to_string_lossy();
        if segment_str == "/sys" {
            break;
        }

        if let Some(bus) = segment_str.strip_prefix("/sys/bus/") {
            if let Some(subsys) = bus.split('/').next() {
                return Ok(subsys.to_string());
            }
        }
    }

    Ok("unknown".to_string())
}

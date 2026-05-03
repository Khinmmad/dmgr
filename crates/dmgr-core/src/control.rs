use crate::error::{DmgrError, Result};
use log::info;

pub fn bind_driver(device_sysfs_path: &str, driver_name: &str) -> Result<()> {
    let driver_bind_path = find_driver_bind_path(device_sysfs_path, driver_name)?;

    let device_name = std::path::Path::new(device_sysfs_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| {
            DmgrError::BindFailed(format!("Invalid device path: {}", device_sysfs_path))
        })?;

    info!(
        "Binding device {} to driver {} at {}",
        device_name, driver_name, driver_bind_path
    );

    std::fs::write(&driver_bind_path, &device_name).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            DmgrError::PermissionDenied(format!(
                "Root required to bind driver. Run with pkexec dmgr-polkit-helper bind {} {}",
                device_sysfs_path, driver_name
            ))
        } else {
            DmgrError::BindFailed(format!("Bind write failed: {}", e))
        }
    })?;

    Ok(())
}

pub fn unbind_driver(device_sysfs_path: &str) -> Result<()> {
    let driver_path = std::path::Path::new(device_sysfs_path).join("driver");
    if !driver_path.exists() {
        return Err(DmgrError::UnbindFailed(
            "Device has no driver bound".to_string(),
        ));
    }

    let driver_name = std::fs::read_link(&driver_path)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .ok_or_else(|| {
            DmgrError::UnbindFailed("Cannot determine driver name".to_string())
        })?;

    let unbind_path = driver_path.join("unbind");

    let device_name = std::path::Path::new(device_sysfs_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| {
            DmgrError::UnbindFailed(format!("Invalid device path: {}", device_sysfs_path))
        })?;

    info!(
        "Unbinding device {} from driver {}",
        device_name, driver_name
    );

    std::fs::write(&unbind_path, &device_name).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            DmgrError::PermissionDenied(format!(
                "Root required to unbind driver. Run with pkexec dmgr-polkit-helper unbind {}",
                device_sysfs_path
            ))
        } else {
            DmgrError::UnbindFailed(format!("Unbind write failed: {}", e))
        }
    })?;

    Ok(())
}

fn find_driver_bind_path(device_sysfs_path: &str, driver_name: &str) -> Result<String> {
    let subsystem = guess_subsystem(device_sysfs_path)?;

    let bind_path = format!("/sys/bus/{}/drivers/{}/bind", subsystem, driver_name);

    if std::path::Path::new(&bind_path).exists() {
        return Ok(bind_path);
    }

    Err(DmgrError::DriverNotFound(format!(
        "Driver '{}' not found for subsystem '{}'",
        driver_name, subsystem
    )))
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

    Err(DmgrError::BindFailed(format!(
        "Cannot determine subsystem for {}",
        device_path
    )))
}

use crate::device::{Bus, Device, DeviceStatus};
use crate::error::{DmgrError, Result};
use log::debug;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const SYSFS_BASE: &str = "/sys";

pub fn scan_all_devices() -> Result<Vec<Device>> {
    let mut devices = Vec::new();
    let mut seen = std::collections::HashSet::new();

    scan_bus_pci(&mut devices, &mut seen)?;
    scan_bus_usb(&mut devices, &mut seen)?;
    scan_class_input(&mut devices, &mut seen)?;
    scan_class_sound(&mut devices, &mut seen)?;
    scan_class_block(&mut devices, &mut seen)?;
    scan_class_drm(&mut devices, &mut seen)?;
    scan_class_net(&mut devices, &mut seen)?;
    scan_class_tty(&mut devices, &mut seen)?;
    scan_class_power(&mut devices, &mut seen)?;

    resolve_relations(&mut devices);

    debug!("Scanned {} devices total", devices.len());
    Ok(devices)
}

fn resolve_relations(devices: &mut Vec<Device>) {
    let mut path_to_id: HashMap<String, String> = HashMap::new();

    for d in devices.iter() {
        path_to_id.insert(d.path.clone(), d.id.clone());
    }

    for d in devices.iter_mut() {
        if let Some(parent_path) = find_parent_path(&d.path) {
            if let Some(parent_id) = path_to_id.get(&parent_path) {
                d.parent = Some(parent_id.clone());
            }
        }
    }

    let mut children_map: HashMap<String, Vec<String>> = HashMap::new();
    for d in devices.iter() {
        if let Some(ref parent_id) = d.parent {
            children_map
                .entry(parent_id.clone())
                .or_default()
                .push(d.id.clone());
        }
    }

    for d in devices.iter_mut() {
        if let Some(kids) = children_map.remove(&d.id) {
            d.children = kids;
        }
    }
}

fn find_parent_path(path: &str) -> Option<String> {
    let p = Path::new(path);
    p.parent()
        .map(|p| p.to_string_lossy().to_string())
        .filter(|s| s.starts_with("/sys/devices"))
}

fn scan_bus_pci(devices: &mut Vec<Device>, seen: &mut std::collections::HashSet<String>) -> Result<()> {
    let pci_devices = Path::new(SYSFS_BASE).join("bus/pci/devices");
    if !pci_devices.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&pci_devices)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        if path.is_symlink() {
            let real_path = fs::read_link(&path)?;
            let canonical = path.parent().unwrap().join(&real_path);
            let canonical = canonical.canonicalize().unwrap_or(canonical);
            let canonical_str = canonical.to_string_lossy().to_string();

            if !seen.insert(canonical_str.clone()) {
                continue;
            }

            if let Ok(mut device) = parse_pci_device(&name, &canonical_str) {
                scan_children(&mut device, devices, seen)?;
                devices.push(device);
            }
        }
    }

    Ok(())
}

fn parse_pci_device(name: &str, path: &str) -> Result<Device> {
    let mut device = Device::new(
        format!("pci-{}", name),
        format!("PCI Device {}", name),
        Bus::Pci,
        "pci".to_string(),
        path.to_string(),
    );

    device.bus_id = Some(name.to_string());
    device.vendor = read_sysfs_file(path, "vendor");
    device.model = read_sysfs_file(path, "device");
    device.vendor_id = read_sysfs_file(path, "vendor").and_then(|v| {
        if v.starts_with("0x") {
            Some(v.trim_start_matches("0x").to_string())
        } else {
            None
        }
    });
    device.model_id = read_sysfs_file(path, "device").and_then(|v| {
        if v.starts_with("0x") {
            Some(v.trim_start_matches("0x").to_string())
        } else {
            None
        }
    });

    device.driver = read_sysfs_link_target(path, "driver");
    device.status = read_device_status(path);
    device.removable = read_sysfs_file(path, "removable").map(|s| s != "fixed").unwrap_or(false);
    device.authorized = read_sysfs_file(path, "authorized")
        .map(|s| s == "1")
        .unwrap_or(true);

    read_uevent_into(path, &mut device);
    read_properties(path, &mut device);

    Ok(device)
}

fn scan_bus_usb(devices: &mut Vec<Device>, seen: &mut std::collections::HashSet<String>) -> Result<()> {
    let usb_devices = Path::new(SYSFS_BASE).join("bus/usb/devices");
    if !usb_devices.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&usb_devices)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        if path.is_symlink() {
            let real_path = fs::read_link(&path)?;
            let canonical = path.parent().unwrap().join(&real_path);
            let canonical = canonical.canonicalize().unwrap_or(canonical);
            let canonical_str = canonical.to_string_lossy().to_string();

            if !seen.insert(canonical_str.clone()) {
                continue;
            }

            if let Ok(mut device) = parse_usb_device(&name, &canonical_str) {
                scan_children(&mut device, devices, seen)?;
                devices.push(device);
            }
        }
    }

    Ok(())
}

fn parse_usb_device(name: &str, path: &str) -> Result<Device> {
    let product = read_sysfs_file(path, "product")
        .or_else(|| read_sysfs_file(path, "idProduct"));
    let display_name = product.unwrap_or_else(|| format!("USB Device {}", name));

    let mut device = Device::new(
        format!("usb-{}", name),
        display_name,
        Bus::Usb,
        "usb".to_string(),
        path.to_string(),
    );

    device.bus_id = Some(name.to_string());
    device.vendor = read_sysfs_file(path, "manufacturer");
    device.model = read_sysfs_file(path, "product");
    device.vendor_id = read_sysfs_file(path, "idVendor");
    device.model_id = read_sysfs_file(path, "idProduct");

    device.driver = read_sysfs_link_target(path, "driver");
    device.status = read_device_status(path);
    device.removable = read_sysfs_file(path, "removable")
        .map(|s| s == "removable")
        .unwrap_or(false);
    device.authorized = read_sysfs_file(path, "authorized")
        .map(|s| s == "1")
        .unwrap_or(true);

    read_uevent_into(path, &mut device);
    read_properties(path, &mut device);

    Ok(device)
}

fn scan_class_input(devices: &mut Vec<Device>, seen: &mut std::collections::HashSet<String>) -> Result<()> {
    scan_class("input", Bus::Input, "input", devices, seen)
}

fn scan_class_sound(devices: &mut Vec<Device>, seen: &mut std::collections::HashSet<String>) -> Result<()> {
    scan_class("sound", Bus::Audio, "sound", devices, seen)
}

fn scan_class_block(devices: &mut Vec<Device>, seen: &mut std::collections::HashSet<String>) -> Result<()> {
    scan_class("block", Bus::Block, "block", devices, seen)
}

fn scan_class_drm(devices: &mut Vec<Device>, seen: &mut std::collections::HashSet<String>) -> Result<()> {
    scan_class("drm", Bus::Drm, "drm", devices, seen)
}

fn scan_class_net(devices: &mut Vec<Device>, seen: &mut std::collections::HashSet<String>) -> Result<()> {
    scan_class("net", Bus::Net, "net", devices, seen)
}

fn scan_class_tty(devices: &mut Vec<Device>, seen: &mut std::collections::HashSet<String>) -> Result<()> {
    scan_class("tty", Bus::Tty, "tty", devices, seen)
}

fn scan_class_power(devices: &mut Vec<Device>, seen: &mut std::collections::HashSet<String>) -> Result<()> {
    scan_class("power_supply", Bus::Power, "power_supply", devices, seen)
}

fn scan_class(
    class: &str,
    bus: Bus,
    subsystem: &str,
    devices: &mut Vec<Device>,
    seen: &mut std::collections::HashSet<String>,
) -> Result<()> {
    let class_path = Path::new(SYSFS_BASE).join("class").join(class);
    if !class_path.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(&class_path)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();

        let real_path = if path.is_symlink() {
            match fs::read_link(&path) {
                Ok(link) => {
                    let resolved = path.parent().unwrap().join(&link);
                    resolved.canonicalize().unwrap_or(resolved)
                }
                Err(_) => path.clone(),
            }
        } else {
            path.clone()
        };

        let canonical_str = real_path.to_string_lossy().to_string();
        if !seen.insert(canonical_str.clone()) {
            continue;
        }

        if let Ok(device) = parse_class_device(&name, &canonical_str, bus.clone(), subsystem) {
            devices.push(device);
        }
    }

    Ok(())
}

fn parse_class_device(
    name: &str,
    path: &str,
    bus: Bus,
    subsystem: &str,
) -> Result<Device> {
    let display_name = format!("{} Device {}", subsystem, name);

    let mut device = Device::new(
        format!("{}-{}", subsystem, name),
        display_name,
        bus.clone(),
        subsystem.to_string(),
        path.to_string(),
    );

    device.driver = read_sysfs_link_target(path, "driver");
    device.status = read_device_status_with_default(path);

    match &bus {
        Bus::Input => {
            if let Some(n) = read_sysfs_file(path, "name") {
                device.name = n;
            }
            device.bus_id = Some(name.to_string());
        }
        Bus::Audio => {
            if let Some(card_name) = read_sysfs_file(path, "id") {
                device.name = format!("Sound Card: {}", card_name);
            } else if let Some(card_num) = read_sysfs_file(path, "number") {
                device.name = format!("Sound Card {}", card_num);
            }
        }
        Bus::Block => {
            if let Some(size_str) = read_sysfs_file(path, "size") {
                if let Ok(sectors) = size_str.trim().parse::<u64>() {
                    let gb = (sectors * 512) as f64 / 1_000_000_000.0;
                    device.name = format!("{} ({:.1} GB)", device.name, gb);
                }
            }
        }
        Bus::Net => {
            if let Some(addr) = read_sysfs_file(path, "address") {
                device.name = format!("NIC {} ({})", name, addr);
            }
        }
        _ => {}
    }

    read_uevent_into(path, &mut device);
    read_properties(path, &mut device);

    Ok(device)
}

fn scan_children(
    parent_device: &mut Device,
    devices: &mut Vec<Device>,
    seen: &mut std::collections::HashSet<String>,
) -> Result<()> {
    let parent_path = Path::new(&parent_device.path);

    let dir_entries = match fs::read_dir(parent_path) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in dir_entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        let child_path = entry.path();
        let child_name = entry.file_name().to_string_lossy().to_string();

        if child_path.is_dir() {
            let child_canonical = match child_path.canonicalize() {
                Ok(c) => c.to_string_lossy().to_string(),
                Err(_) => continue,
            };

            if seen.contains(&child_canonical) {
                continue;
            }

            seen.insert(child_canonical.clone());

            let child_uevent_path = child_path.join("uevent");
            if child_uevent_path.exists() {
                if let Ok(mut child_device) = parse_generic_device(&child_name, &child_canonical) {
                    child_device.parent = Some(parent_device.id.clone());
                    devices.push(child_device);
                }
            }
        }
    }

    Ok(())
}

fn parse_generic_device(name: &str, path: &str) -> Result<Device> {
    let subsystem = read_sysfs_file(path, "subsystem")
        .and_then(|s| {
            Path::new(&s)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let bus = Bus::from_subsystem(&subsystem);

    let mut device = Device::new(
        format!("dev-{}", name),
        name.to_string(),
        bus,
        subsystem,
        path.to_string(),
    );

    read_uevent_into(path, &mut device);
    read_properties(path, &mut device);
    device.driver = read_sysfs_link_target(path, "driver");
    device.status = read_device_status(path);

    Ok(device)
}

fn read_device_status(path: &str) -> DeviceStatus {
    let runtime_status = read_sysfs_file(path, "power/runtime_status");
    if let Some(status) = runtime_status {
        return DeviceStatus::from_str(&status);
    }

    let offline = read_sysfs_file(path, "online");
    if let Some(val) = offline {
        if val == "0" {
            return DeviceStatus::Offline;
        }
    }

    let has_driver = read_sysfs_link_target(path, "driver").is_some();
    if !has_driver {
        return DeviceStatus::Unbound;
    }

    DeviceStatus::Online
}

fn read_device_status_with_default(path: &str) -> DeviceStatus {
    let runtime_status = read_sysfs_file(path, "power/runtime_status");
    if let Some(status) = runtime_status {
        return DeviceStatus::from_str(&status);
    }

    let offline = read_sysfs_file(path, "online");
    if let Some(val) = offline {
        if val == "0" {
            return DeviceStatus::Offline;
        }
    }

    let has_driver = read_sysfs_link_target(path, "driver").is_some();
    if !has_driver {
        return DeviceStatus::Online;
    }

    DeviceStatus::Online
}

fn read_uevent_into(path: &str, device: &mut Device) {
    let uevent = Path::new(path).join("uevent");
    if let Ok(content) = fs::read_to_string(&uevent) {
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                match key {
                    "DRIVER" => {
                        if device.driver.is_none() {
                            device.driver = Some(value.to_string());
                        }
                    }
                    "PCI_CLASS" => {
                        if device.vendor.is_none() {
                            device.vendor = Some(value.to_string());
                        }
                    }
                    "MODALIAS" => {
                        parse_modalias(value, device);
                    }
                    _ => {}
                }
            }
        }
    }
}

fn parse_modalias(modalias: &str, device: &mut Device) {
    for part in modalias.split(':') {
        if let Some(rest) = part.strip_prefix('v') {
            if rest.len() == 8 {
                device.vendor_id = Some(rest.to_string());
            }
        } else if let Some(rest) = part.strip_prefix('d') {
            if rest.len() == 8 {
                device.model_id = Some(rest.to_string());
            }
        }
    }
}

fn read_properties(path: &str, device: &mut Device) {
    let p = Path::new(path);

    let power_path = p.join("power");
    if power_path.exists() {
        if let Ok(entries) = fs::read_dir(&power_path) {
            for entry in entries.flatten() {
                let prop_name = entry.file_name().to_string_lossy().to_string();
                let prop_path = entry.path();
                if let Ok(val) = fs::read_to_string(&prop_path) {
                    let key = format!("power/{}", prop_name);
                    if prop_name == "control" || prop_name == "async" {
                        device.editable_properties.push(key.clone());
                    }
                    device.properties.insert(key, val.trim().to_string());
                }
            }
        }
    }

    for prop in ["removable", "authorized", "iommu_group", "numa_node"].iter() {
        if let Some(val) = read_sysfs_file(path, prop) {
            let val_clone = val.clone();
            device.properties.insert(prop.to_string(), val);
            if *prop == "authorized" {
                device.editable_properties.push(prop.to_string());
                device.authorized = val_clone == "1";
            }
            if *prop == "removable" {
                device.removable = read_sysfs_file(path, "removable")
                    .map(|s| s != "fixed")
                    .unwrap_or(false);
            }
        }
    }
}

pub fn read_sysfs_file(path: &str, name: &str) -> Option<String> {
    let file_path = Path::new(path).join(name);
    fs::read_to_string(&file_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn read_sysfs_link_target(path: &str, name: &str) -> Option<String> {
    let link_path = Path::new(path).join(name);
    fs::read_link(&link_path)
        .ok()
        .map(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .flatten()
}

pub fn write_sysfs_file(path: &str, name: &str, value: &str) -> Result<()> {
    let file_path = Path::new(path).join(name);
    fs::write(&file_path, value).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            DmgrError::PermissionDenied(format!(
                "Cannot write to {}: {}",
                file_path.display(),
                e
            ))
        } else {
            DmgrError::Io(e)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_does_not_crash() {
        let devices = scan_all_devices();
        assert!(devices.is_ok());
        println!("Found {} devices", devices.unwrap().len());
    }

    #[test]
    fn test_bus_from_subsystem() {
        assert_eq!(Bus::from_subsystem("usb"), Bus::Usb);
        assert_eq!(Bus::from_subsystem("pci"), Bus::Pci);
        assert_eq!(Bus::from_subsystem("sound"), Bus::Audio);
        assert_eq!(Bus::from_subsystem("input"), Bus::Input);
        assert_eq!(Bus::from_subsystem("block"), Bus::Block);
        assert_eq!(Bus::from_subsystem("drm"), Bus::Drm);
        assert_eq!(Bus::from_subsystem("net"), Bus::Net);
        assert_eq!(Bus::from_subsystem("hid"), Bus::Hid);
        assert!(matches!(Bus::from_subsystem("foobar"), Bus::Unknown(_)));
    }
}

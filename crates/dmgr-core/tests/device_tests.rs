use dmgr_core::device::{Bus, Device, DeviceStatus};
use dmgr_core::sysfs;

#[test]
fn integration_bus_display() {
    assert_eq!(format!("{}", Bus::Usb), "USB");
    assert_eq!(format!("{}", Bus::Pci), "PCIe");
    assert_eq!(format!("{}", Bus::Audio), "Audio");
    assert_eq!(format!("{}", Bus::Drm), "GPU/DRM");
    assert_eq!(format!("{}", Bus::Net), "Network");
    assert_eq!(format!("{}", Bus::Unknown("custom".into())), "custom");
}

#[test]
fn integration_device_status_mapping() {
    assert_eq!(DeviceStatus::from_str("active"), DeviceStatus::Online);
    assert_eq!(DeviceStatus::from_str("suspended"), DeviceStatus::Suspended);
    assert_eq!(DeviceStatus::from_str("offline"), DeviceStatus::Offline);
    assert_eq!(DeviceStatus::from_str("error_something"), DeviceStatus::Error);
    assert_eq!(DeviceStatus::from_str("whatever"), DeviceStatus::Online);
    assert_eq!(DeviceStatus::from_str("unsupported"), DeviceStatus::Online);
    assert_eq!(DeviceStatus::from_str(""), DeviceStatus::Online);
}

#[test]
fn integration_device_status_emoji() {
    assert_eq!(DeviceStatus::Online.emoji(), "🟢");
    assert_eq!(DeviceStatus::Offline.emoji(), "🔴");
    assert_eq!(DeviceStatus::Suspended.emoji(), "🟡");
    assert_eq!(DeviceStatus::Unbound.emoji(), "⚪");
    assert_eq!(DeviceStatus::Error.emoji(), "🟠");
}

#[test]
fn integration_device_creation() {
    let dev = Device::new(
        "test-001".into(),
        "Test Device".into(),
        Bus::Usb,
        "usb".into(),
        "/sys/devices/test".into(),
    );
    assert_eq!(dev.id, "test-001");
    assert_eq!(dev.name, "Test Device");
    assert_eq!(dev.bus, Bus::Usb);
    assert_eq!(dev.status, DeviceStatus::Unknown);
    assert!(dev.properties.is_empty());
    assert!(dev.children.is_empty());
}

#[test]
fn integration_device_json_roundtrip() {
    let dev = Device {
        id: "usb-1-2".into(),
        name: "USB Webcam".into(),
        bus: Bus::Usb,
        bus_id: Some("1-2".into()),
        vendor: Some("Logitech".into()),
        vendor_id: Some("046d".into()),
        model: Some("HD Pro Webcam C920".into()),
        model_id: Some("082d".into()),
        driver: Some("uvcvideo".into()),
        status: DeviceStatus::Online,
        subsystem: "usb".into(),
        path: "/sys/devices/pci0000:00/0000:00:14.0/usb1/1-2".into(),
        parent: None,
        children: vec![],
        interfaces: vec![],
        properties: std::collections::HashMap::new(),
        editable_properties: vec![],
        removable: true,
        authorized: true,
    };

    let json = serde_json::to_string(&dev).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed["id"], "usb-1-2");
    assert_eq!(parsed["name"], "USB Webcam");
    assert_eq!(parsed["bus"], "Usb");
    assert_eq!(parsed["driver"], "uvcvideo");
    assert_eq!(parsed["status"], "Online");
    assert_eq!(parsed["vendor_id"], "046d");
    assert_eq!(parsed["removable"], true);

    let deserialized: Device = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.id, dev.id);
    assert_eq!(deserialized.name, dev.name);
    assert_eq!(deserialized.bus, dev.bus);
}

#[test]
fn integration_scan_finds_devices() {
    let result = sysfs::scan_all_devices();
    assert!(result.is_ok(), "Scanner should not crash");
    let devices = result.unwrap();
    assert!(!devices.is_empty(), "Should find at least some devices on a Linux system");
}

#[test]
fn integration_pci_devices_have_required_fields() {
    if let Ok(devices) = sysfs::scan_all_devices() {
        let pci_devices: Vec<_> = devices.iter().filter(|d| d.bus == Bus::Pci).collect();
        for d in &pci_devices {
            assert!(!d.id.is_empty(), "PCI device must have ID");
            assert!(!d.path.is_empty(), "PCI device must have path");
            assert!(d.path.contains("/sys/devices"), "Path must be under /sys/devices");
        }
    }
}

#[test]
fn integration_cpu_modalias_readable() {
    let modalias = sysfs::read_sysfs_file("/sys/devices/system/cpu/cpu0", "modalias");
    if let Some(m) = &modalias {
        assert!(!m.is_empty(), "modalias should not be empty if present");
    }
}

#[test]
fn integration_get_drivers_does_not_panic() {
    let result = dmgr_core::properties::get_available_drivers("/sys/devices/system/cpu/cpu0");
    assert!(result.is_ok(), "get_available_drivers should not panic");
}

#[test]
fn integration_unbind_on_cpu_fails() {
    let result = dmgr_core::control::unbind_driver("/sys/devices/system/cpu/cpu0");
    assert!(result.is_err(), "Unbinding CPU should fail");
}

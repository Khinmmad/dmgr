use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Bus {
    Usb,
    Pci,
    Audio,
    Input,
    Block,
    Drm,
    Net,
    Hid,
    Tty,
    Power,
    IoMMU,
    Unknown(String),
}

impl fmt::Display for Bus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Bus::Usb => write!(f, "USB"),
            Bus::Pci => write!(f, "PCIe"),
            Bus::Audio => write!(f, "Audio"),
            Bus::Input => write!(f, "Input"),
            Bus::Block => write!(f, "Block"),
            Bus::Drm => write!(f, "GPU/DRM"),
            Bus::Net => write!(f, "Network"),
            Bus::Hid => write!(f, "HID"),
            Bus::Tty => write!(f, "TTY"),
            Bus::Power => write!(f, "Power"),
            Bus::IoMMU => write!(f, "IOMMU"),
            Bus::Unknown(s) => write!(f, "{}", s),
        }
    }
}

impl Bus {
    pub fn from_subsystem(s: &str) -> Self {
        match s {
            "usb" | "usb_device" => Bus::Usb,
            "pci" => Bus::Pci,
            "sound" | "audio" => Bus::Audio,
            "input" => Bus::Input,
            "block" => Bus::Block,
            "drm" => Bus::Drm,
            "net" => Bus::Net,
            "hid" => Bus::Hid,
            "tty" => Bus::Tty,
            "power_supply" => Bus::Power,
            "iommu" => Bus::IoMMU,
            other => Bus::Unknown(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceStatus {
    Online,
    Offline,
    Suspended,
    Unbound,
    Error,
    Unknown,
}

impl fmt::Display for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeviceStatus::Online => write!(f, "Online"),
            DeviceStatus::Offline => write!(f, "Offline"),
            DeviceStatus::Suspended => write!(f, "Suspended"),
            DeviceStatus::Unbound => write!(f, "Unbound"),
            DeviceStatus::Error => write!(f, "Error"),
            DeviceStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

impl DeviceStatus {
    pub fn from_str(s: &str) -> Self {
        match s {
            "active" | "online" | "on" => DeviceStatus::Online,
            "suspended" | "suspend" => DeviceStatus::Suspended,
            "offline" | "off" => DeviceStatus::Offline,
            other if other.contains("error") || other.contains("fail") => DeviceStatus::Error,
            _ => DeviceStatus::Unknown,
        }
    }

    pub fn emoji(&self) -> &str {
        match self {
            DeviceStatus::Online => "🟢",
            DeviceStatus::Offline => "🔴",
            DeviceStatus::Suspended => "🟡",
            DeviceStatus::Unbound => "⚪",
            DeviceStatus::Error => "🟠",
            DeviceStatus::Unknown => "⚫",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interface {
    pub name: String,
    pub class: String,
    pub protocol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    pub bus: Bus,
    pub bus_id: Option<String>,
    pub vendor: Option<String>,
    pub vendor_id: Option<String>,
    pub model: Option<String>,
    pub model_id: Option<String>,
    pub driver: Option<String>,
    pub status: DeviceStatus,
    pub subsystem: String,
    pub path: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub interfaces: Vec<Interface>,
    pub properties: std::collections::HashMap<String, String>,
    pub editable_properties: Vec<String>,
    pub removable: bool,
    pub authorized: bool,
}

impl Device {
    pub fn new(id: String, name: String, bus: Bus, subsystem: String, path: String) -> Self {
        Device {
            id,
            name,
            bus,
            bus_id: None,
            vendor: None,
            vendor_id: None,
            model: None,
            model_id: None,
            driver: None,
            status: DeviceStatus::Unknown,
            subsystem,
            path,
            parent: None,
            children: Vec::new(),
            interfaces: Vec::new(),
            properties: std::collections::HashMap::new(),
            editable_properties: Vec::new(),
            removable: false,
            authorized: true,
        }
    }
}

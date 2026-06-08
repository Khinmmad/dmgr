//! Advanced per-device details read straight from sysfs (no lspci/lsusb needed,
//! so it works on any Linux). Curated per bus: PCIe link speed/width, IRQ, USB
//! version/speed/power, runtime PM, etc.

use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct DetailItem {
    pub label: String,
    pub value: String,
}

fn read(base: &str, rel: &str) -> Option<String> {
    let v = std::fs::read_to_string(Path::new(base).join(rel)).ok()?;
    let v = v.trim().to_string();
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

fn push(out: &mut Vec<DetailItem>, label: &str, base: &str, rel: &str) {
    if let Some(value) = read(base, rel) {
        out.push(DetailItem {
            label: label.to_string(),
            value,
        });
    }
}

/// `bus` is the serialized Bus string ("Pci", "Usb", ...).
pub fn advanced(path: &str, bus: &str) -> Vec<DetailItem> {
    let mut out = Vec::new();

    match bus {
        "Pci" => {
            push(&mut out, "Current link speed", path, "current_link_speed");
            push(&mut out, "Max link speed", path, "max_link_speed");
            push(&mut out, "Current link width", path, "current_link_width");
            push(&mut out, "Max link width", path, "max_link_width");
            push(&mut out, "IRQ", path, "irq");
            push(&mut out, "NUMA node", path, "numa_node");
            push(&mut out, "Class", path, "class");
            push(&mut out, "Revision", path, "revision");
            push(&mut out, "Enabled", path, "enable");
            push(&mut out, "D3cold allowed", path, "d3cold_allowed");
            // MSI interrupt count
            if let Ok(rd) = std::fs::read_dir(Path::new(path).join("msi_irqs")) {
                let n = rd.flatten().count();
                if n > 0 {
                    out.push(DetailItem {
                        label: "MSI IRQs".into(),
                        value: n.to_string(),
                    });
                }
            }
        }
        "Usb" => {
            push(&mut out, "USB version", path, "version");
            push(&mut out, "Speed (Mbps)", path, "speed");
            push(&mut out, "Max power", path, "bMaxPower");
            push(&mut out, "Device class", path, "bDeviceClass");
            push(&mut out, "Configurations", path, "bNumConfigurations");
            push(&mut out, "Interfaces", path, "bNumInterfaces");
            push(&mut out, "Ports (hub)", path, "maxchild");
            push(&mut out, "Bus number", path, "busnum");
            push(&mut out, "Device number", path, "devnum");
            push(&mut out, "Serial", path, "serial");
            push(&mut out, "Autosuspend (ms)", path, "power/autosuspend_delay_ms");
        }
        "Net" => {
            push(&mut out, "MAC address", path, "address");
            push(&mut out, "Speed (Mbps)", path, "speed");
            push(&mut out, "Operstate", path, "operstate");
            push(&mut out, "MTU", path, "mtu");
            push(&mut out, "Carrier", path, "carrier");
            push(&mut out, "Duplex", path, "duplex");
        }
        "Block" => {
            push(&mut out, "Size (sectors)", path, "size");
            push(&mut out, "Read-only", path, "ro");
            push(&mut out, "Removable", path, "removable");
            push(&mut out, "Rotational", path, "queue/rotational");
            push(&mut out, "Scheduler", path, "queue/scheduler");
            push(&mut out, "Model", path, "device/model");
        }
        "Drm" => {
            push(&mut out, "Enabled", path, "enabled");
            push(&mut out, "Status", path, "status");
            push(&mut out, "DPMS", path, "dpms");
        }
        _ => {}
    }

    // Power management — common to most subsystems.
    push(&mut out, "Runtime PM control", path, "power/control");
    push(&mut out, "Runtime status", path, "power/runtime_status");
    push(&mut out, "Wakeup", path, "power/wakeup");
    push(&mut out, "Modalias", path, "modalias");

    out
}

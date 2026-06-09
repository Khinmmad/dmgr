//! Audio enumeration & switching, abstracted over the available sound stack.
//! Autodetects, in order: pactl (PipeWire-pulse / PulseAudio) → wpctl (WirePlumber)
//! → ALSA (read-only fallback).

#[cfg(not(target_os = "windows"))]
mod alsa;
#[cfg(not(target_os = "windows"))]
mod pactl;
#[cfg(target_os = "windows")]
mod wasapi;
#[cfg(not(target_os = "windows"))]
mod wpctl;

use serde::Serialize;
#[cfg(not(target_os = "windows"))]
use std::process::Command;
#[cfg(not(target_os = "windows"))]
use std::sync::OnceLock;

#[derive(Clone, Debug, Serialize)]
pub struct AudioDevice {
    pub index: u32,
    /// Opaque switching key for the active backend (pactl sink name / wpctl node id / alsa hw).
    pub name: String,
    pub description: String,
    pub state: String, // RUNNING | SUSPENDED | IDLE
    pub muted: bool,
    pub volume: Option<u32>, // percent
    pub is_default: bool,
    pub kind: String, // Builtin | Usb | Hdmi | Bluetooth | Virtual
}

/// One backend's worth of audio control. `id` is the opaque `AudioDevice.name`.
pub trait AudioBackend: Send + Sync {
    fn name(&self) -> &'static str;
    fn outputs(&self) -> Vec<AudioDevice>;
    fn inputs(&self) -> Vec<AudioDevice>;
    fn set_default_output(&self, id: &str) -> Result<(), String>;
    fn set_default_input(&self, id: &str) -> Result<(), String>;
    fn set_volume(&self, id: &str, percent: u32) -> Result<(), String>;
    fn set_mute(&self, id: &str, muted: bool) -> Result<(), String>;
}

#[cfg(not(target_os = "windows"))]
#[derive(Clone, Copy)]
enum Kind {
    Pactl,
    Wpctl,
    Alsa,
}

#[cfg(not(target_os = "windows"))]
fn has(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn choice() -> Option<Kind> {
    static CHOICE: OnceLock<Option<Kind>> = OnceLock::new();
    *CHOICE.get_or_init(|| {
        if has("pactl") {
            Some(Kind::Pactl)
        } else if has("wpctl") {
            Some(Kind::Wpctl)
        } else if alsa::available() {
            Some(Kind::Alsa)
        } else {
            None
        }
    })
}

#[cfg(not(target_os = "windows"))]
pub fn detect() -> Option<Box<dyn AudioBackend>> {
    Some(match choice()? {
        Kind::Pactl => Box::new(pactl::Pactl),
        Kind::Wpctl => Box::new(wpctl::Wpctl),
        Kind::Alsa => Box::new(alsa::Alsa),
    })
}

/// Windows always has Core Audio (WASAPI).
#[cfg(target_os = "windows")]
pub fn detect() -> Option<Box<dyn AudioBackend>> {
    Some(Box::new(wasapi::Wasapi))
}

pub fn is_available() -> bool {
    detect().is_some()
}

/// Name of the active audio backend (for diagnostics / the UI).
pub fn backend_name() -> &'static str {
    detect().map(|b| b.name()).unwrap_or("none")
}

// ── shared helpers used by impls ────────────────────────────────────────────

pub(crate) fn detect_kind(name: &str) -> &'static str {
    let n = name.to_lowercase();
    if n.contains("bluez") || n.contains("bluetooth") {
        "Bluetooth"
    } else if n.contains("hdmi") || n.contains("iec958") || n.contains("spdif") {
        "Hdmi"
    } else if n.contains("usb") {
        "Usb"
    } else if n.contains("null") || n.contains("virtual") || n.contains("pipewire") {
        "Virtual"
    } else {
        "Builtin"
    }
}

#[cfg(test)]
mod tests {
    use super::detect_kind;

    #[test]
    fn classifies_device_kinds() {
        assert_eq!(detect_kind("bluez_output.AA_BB_CC.1"), "Bluetooth");
        assert_eq!(detect_kind("alsa_output.pci-0000_01.hdmi-stereo"), "Hdmi");
        assert_eq!(detect_kind("alsa_output.usb-Generic_USB"), "Usb");
        assert_eq!(detect_kind("alsa_output.pci-0000_0c.analog-stereo"), "Builtin");
        assert_eq!(detect_kind("alsa_output.virtual-sink"), "Virtual");
    }
}

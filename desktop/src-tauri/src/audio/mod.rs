//! Audio enumeration & switching, abstracted over the available sound stack.
//! Autodetects, in order: pactl (PipeWire-pulse / PulseAudio) → wpctl (WirePlumber)
//! → ALSA (read-only fallback).

mod alsa;
mod pactl;
mod wpctl;

use serde::Serialize;
use std::process::Command;
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

#[derive(Clone, Copy)]
enum Kind {
    Pactl,
    Wpctl,
    Alsa,
}

fn has(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

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

pub fn detect() -> Option<Box<dyn AudioBackend>> {
    Some(match choice()? {
        Kind::Pactl => Box::new(pactl::Pactl),
        Kind::Wpctl => Box::new(wpctl::Wpctl),
        Kind::Alsa => Box::new(alsa::Alsa),
    })
}

pub fn is_available() -> bool {
    choice().is_some()
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

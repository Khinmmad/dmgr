use log::warn;
use std::process::Command;

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct AudioDevice {
    #[allow(dead_code)]
    pub index: u32,
    pub name: String,         // internal pactl name (used for switching)
    pub description: String,  // human-readable display name
    pub state: String,        // RUNNING | SUSPENDED | IDLE
    pub muted: bool,
    pub is_default: bool,
    pub kind: AudioKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AudioKind {
    Builtin,
    Usb,
    Hdmi,
    Bluetooth,
    Virtual,
}

impl AudioDevice {
    pub fn icon(&self) -> &'static str {
        match self.kind {
            AudioKind::Bluetooth => "🔵",
            AudioKind::Hdmi => "🖥",
            AudioKind::Usb => "🔌",
            AudioKind::Virtual => "⬡",
            AudioKind::Builtin => "🔊",
        }
    }

    pub fn state_color(&self) -> egui::Color32 {
        match self.state.as_str() {
            "RUNNING" => egui::Color32::from_rgb(80, 200, 120),
            "IDLE" => egui::Color32::from_rgb(180, 180, 80),
            "SUSPENDED" => egui::Color32::from_rgb(120, 120, 120),
            _ => egui::Color32::from_rgb(100, 100, 100),
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────────────

pub fn list_sinks() -> Vec<AudioDevice> {
    let default_name = default_sink().unwrap_or_default();
    match run_pactl(&["list", "sinks"]) {
        Some(text) => parse_devices(&text, "Sink", &default_name),
        None => Vec::new(),
    }
}

pub fn list_sources() -> Vec<AudioDevice> {
    let default_name = default_source().unwrap_or_default();
    match run_pactl(&["list", "sources"]) {
        Some(text) => parse_devices(&text, "Source", &default_name)
            .into_iter()
            .filter(|d| !d.name.ends_with(".monitor") && !d.description.starts_with("Monitor of"))
            .collect(),
        None => Vec::new(),
    }
}

pub fn set_default_sink(name: &str) -> Result<(), String> {
    run_pactl_cmd(&["set-default-sink", name])
}

pub fn set_default_source(name: &str) -> Result<(), String> {
    run_pactl_cmd(&["set-default-source", name])
}

pub fn is_available() -> bool {
    Command::new("pactl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

// ── Internals ─────────────────────────────────────────────────────────────────

fn default_sink() -> Option<String> {
    run_pactl(&["get-default-sink"]).map(|s| s.trim().to_string())
}

fn default_source() -> Option<String> {
    run_pactl(&["get-default-source"]).map(|s| s.trim().to_string())
}

fn run_pactl(args: &[&str]) -> Option<String> {
    let out = Command::new("pactl").args(args).output().ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout).ok()
    } else {
        warn!(
            "pactl {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr)
        );
        None
    }
}

fn run_pactl_cmd(args: &[&str]) -> Result<(), String> {
    let status = Command::new("pactl")
        .args(args)
        .status()
        .map_err(|e| format!("pactl not found: {}", e))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("pactl {} failed (exit {})", args.join(" "), status.code().unwrap_or(-1)))
    }
}

/// Parse `pactl list sinks` or `pactl list sources` text output.
fn parse_devices(text: &str, kind: &str, default_name: &str) -> Vec<AudioDevice> {
    let mut devices = Vec::new();
    let block_prefix = format!("{} #", kind);

    // Split text into per-device blocks
    let mut blocks: Vec<&str> = Vec::new();
    let mut start = 0;
    let lines: Vec<&str> = text.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with(&block_prefix) && i > start {
            blocks.push(&text[text
                .lines()
                .take(start)
                .map(|l| l.len() + 1)
                .sum::<usize>()..]);
            start = i;
        }
    }
    // Push the remaining text from `start` to end
    let skip = lines.iter().take(start).map(|l| l.len() + 1).sum::<usize>();
    if skip < text.len() {
        blocks.push(&text[skip..]);
    }

    for block in blocks {
        if !block.trim_start().starts_with(&block_prefix) {
            continue;
        }

        let mut index = 0u32;
        let mut name = String::new();
        let mut description = String::new();
        let mut state = String::new();
        let mut muted = false;

        for line in block.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with(&block_prefix) {
                // e.g. "Sink #64"
                let rest = &trimmed[block_prefix.len()..];
                index = rest.trim().parse().unwrap_or(0);
            } else if let Some(v) = trimmed.strip_prefix("Name: ") {
                name = v.to_string();
            } else if let Some(v) = trimmed.strip_prefix("Description: ") {
                description = v.to_string();
            } else if let Some(v) = trimmed.strip_prefix("State: ") {
                state = v.to_string();
            } else if let Some(v) = trimmed.strip_prefix("Mute: ") {
                muted = v.trim() == "yes";
            }
        }

        if name.is_empty() {
            continue;
        }
        if description.is_empty() {
            description = name.clone();
        }

        let kind = detect_kind(&name);

        devices.push(AudioDevice {
            index,
            is_default: name == default_name,
            name,
            description,
            state,
            muted,
            kind,
        });
    }

    devices
}

fn detect_kind(name: &str) -> AudioKind {
    let n = name.to_lowercase();
    if n.contains("bluez") || n.contains("bluetooth") {
        AudioKind::Bluetooth
    } else if n.contains("hdmi") || n.contains("iec958") || n.contains("spdif") {
        AudioKind::Hdmi
    } else if n.contains("usb") {
        AudioKind::Usb
    } else if n.contains("null") || n.contains("virtual") || n.contains("pipewire") {
        AudioKind::Virtual
    } else {
        AudioKind::Builtin
    }
}

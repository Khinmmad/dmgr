//! Audio device enumeration & default switching via `pactl` (PipeWire-pulse / PulseAudio).

use serde::Serialize;
use std::process::Command;

#[derive(Clone, Debug, Serialize)]
pub struct AudioDevice {
    pub index: u32,
    pub name: String,        // internal pactl name (used for switching)
    pub description: String, // human-readable display name
    pub state: String,       // RUNNING | SUSPENDED | IDLE
    pub muted: bool,
    pub volume: Option<u32>, // percent, best-effort
    pub is_default: bool,
    pub kind: String, // Builtin | Usb | Hdmi | Bluetooth | Virtual
}

pub fn is_available() -> bool {
    Command::new("pactl")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn list_sinks() -> Vec<AudioDevice> {
    let default_name = default_of("get-default-sink");
    match run(&["list", "sinks"]) {
        Some(text) => parse(&text, "Sink", &default_name),
        None => Vec::new(),
    }
}

pub fn list_sources() -> Vec<AudioDevice> {
    let default_name = default_of("get-default-source");
    match run(&["list", "sources"]) {
        Some(text) => parse(&text, "Source", &default_name)
            .into_iter()
            .filter(|d| !d.name.ends_with(".monitor") && !d.description.starts_with("Monitor of"))
            .collect(),
        None => Vec::new(),
    }
}

pub fn set_default_sink(name: &str) -> Result<(), String> {
    run_cmd(&["set-default-sink", name])
}

pub fn set_default_source(name: &str) -> Result<(), String> {
    run_cmd(&["set-default-source", name])
}

pub fn set_sink_volume(name: &str, percent: u32) -> Result<(), String> {
    let p = percent.min(150);
    run_cmd(&["set-sink-volume", name, &format!("{p}%")])
}

pub fn set_sink_mute(name: &str, muted: bool) -> Result<(), String> {
    run_cmd(&["set-sink-mute", name, if muted { "1" } else { "0" }])
}

// ── internals ───────────────────────────────────────────────────────────────

fn default_of(cmd: &str) -> String {
    run(&[cmd]).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn run(args: &[&str]) -> Option<String> {
    let out = Command::new("pactl").args(args).output().ok()?;
    if out.status.success() {
        String::from_utf8(out.stdout).ok()
    } else {
        None
    }
}

fn run_cmd(args: &[&str]) -> Result<(), String> {
    let status = Command::new("pactl")
        .args(args)
        .status()
        .map_err(|e| format!("pactl not found: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "pactl {} failed (exit {})",
            args.join(" "),
            status.code().unwrap_or(-1)
        ))
    }
}

fn parse(text: &str, kind: &str, default_name: &str) -> Vec<AudioDevice> {
    let mut devices = Vec::new();
    let block_prefix = format!("{kind} #");

    let mut current: Option<AudioDevice> = None;
    let push = |c: &mut Option<AudioDevice>, list: &mut Vec<AudioDevice>| {
        if let Some(d) = c.take() {
            if !d.name.is_empty() {
                list.push(d);
            }
        }
    };

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&block_prefix) {
            push(&mut current, &mut devices);
            let idx = trimmed[block_prefix.len()..].trim().parse().unwrap_or(0);
            current = Some(AudioDevice {
                index: idx,
                name: String::new(),
                description: String::new(),
                state: String::new(),
                muted: false,
                volume: None,
                is_default: false,
                kind: "Builtin".into(),
            });
        } else if let Some(d) = current.as_mut() {
            if let Some(v) = trimmed.strip_prefix("Name: ") {
                d.name = v.to_string();
                d.kind = detect_kind(v).to_string();
                d.is_default = v == default_name;
            } else if let Some(v) = trimmed.strip_prefix("Description: ") {
                d.description = v.to_string();
            } else if let Some(v) = trimmed.strip_prefix("State: ") {
                d.state = v.to_string();
            } else if let Some(v) = trimmed.strip_prefix("Mute: ") {
                d.muted = v.trim() == "yes";
            } else if d.volume.is_none() && trimmed.starts_with("Volume:") {
                d.volume = parse_volume(trimmed);
            }
        }
    }
    push(&mut current, &mut devices);

    for d in devices.iter_mut() {
        if d.description.is_empty() {
            d.description = d.name.clone();
        }
    }
    devices
}

fn parse_volume(line: &str) -> Option<u32> {
    // e.g. "Volume: front-left: 45000 / 69% / -9.33 dB, ..."
    line.split('/')
        .map(|s| s.trim())
        .find(|s| s.ends_with('%'))
        .and_then(|s| s.trim_end_matches('%').trim().parse::<u32>().ok())
}

fn detect_kind(name: &str) -> &'static str {
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

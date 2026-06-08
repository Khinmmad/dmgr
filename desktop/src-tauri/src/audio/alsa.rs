//! ALSA fallback — lists hardware devices via `aplay`/`arecord`. Read-only:
//! switching the default device at runtime isn't an ALSA concept, so the set
//! operations return a clear hint to install PipeWire/PulseAudio.

use super::{detect_kind, AudioBackend, AudioDevice};
use std::process::Command;

pub struct Alsa;

const READ_ONLY: &str =
    "ALSA backend is read-only. Install PipeWire (wpctl) or PulseAudio (pactl) to switch devices.";

pub fn available() -> bool {
    Command::new("aplay")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

impl AudioBackend for Alsa {
    fn name(&self) -> &'static str {
        "alsa"
    }

    fn outputs(&self) -> Vec<AudioDevice> {
        list("aplay")
    }
    fn inputs(&self) -> Vec<AudioDevice> {
        list("arecord")
    }

    fn set_default_output(&self, _: &str) -> Result<(), String> {
        Err(READ_ONLY.into())
    }
    fn set_default_input(&self, _: &str) -> Result<(), String> {
        Err(READ_ONLY.into())
    }
    fn set_volume(&self, _: &str, _: u32) -> Result<(), String> {
        Err(READ_ONLY.into())
    }
    fn set_mute(&self, _: &str, _: bool) -> Result<(), String> {
        Err(READ_ONLY.into())
    }
}

fn list(tool: &str) -> Vec<AudioDevice> {
    let out = match Command::new(tool).arg("-l").output() {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).into_owned(),
        _ => return Vec::new(),
    };

    let mut devices = Vec::new();
    for line in out.lines() {
        // "card 0: PCH [HDA Intel PCH], device 0: ALC892 Analog [ALC892 Analog]"
        let line = line.trim();
        if !line.starts_with("card ") {
            continue;
        }
        let Some(card) = field_num(line, "card ") else { continue };
        let device = field_num(line, "device ").unwrap_or(0);
        let desc = line
            .split(", device")
            .next()
            .and_then(|s| s.split_once(':'))
            .map(|(_, n)| n.trim().to_string())
            .unwrap_or_else(|| line.to_string());

        devices.push(AudioDevice {
            index: card,
            name: format!("hw:{card},{device}"),
            description: desc.clone(),
            state: String::new(),
            muted: false,
            volume: None,
            is_default: card == 0 && device == 0,
            kind: detect_kind(&desc).to_string(),
        });
    }
    devices
}

fn field_num(line: &str, key: &str) -> Option<u32> {
    let rest = &line[line.find(key)? + key.len()..];
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

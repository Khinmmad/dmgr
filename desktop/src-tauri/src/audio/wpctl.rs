//! WirePlumber backend — parses `wpctl status`, switches by node id.

use super::{detect_kind, AudioBackend, AudioDevice};
use std::process::Command;

pub struct Wpctl;

impl AudioBackend for Wpctl {
    fn name(&self) -> &'static str {
        "wpctl"
    }

    fn outputs(&self) -> Vec<AudioDevice> {
        status().map(|t| section(&t, "Sinks")).unwrap_or_default()
    }

    fn inputs(&self) -> Vec<AudioDevice> {
        status().map(|t| section(&t, "Sources")).unwrap_or_default()
    }

    fn set_default_output(&self, id: &str) -> Result<(), String> {
        cmd(&["set-default", id])
    }
    fn set_default_input(&self, id: &str) -> Result<(), String> {
        cmd(&["set-default", id])
    }
    fn set_volume(&self, id: &str, percent: u32) -> Result<(), String> {
        cmd(&["set-volume", id, &format!("{:.2}", (percent.min(150) as f32) / 100.0)])
    }
    fn set_mute(&self, id: &str, muted: bool) -> Result<(), String> {
        cmd(&["set-mute", id, if muted { "1" } else { "0" }])
    }
}

fn status() -> Option<String> {
    let out = Command::new("wpctl").arg("status").output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn cmd(args: &[&str]) -> Result<(), String> {
    let st = Command::new("wpctl")
        .args(args)
        .status()
        .map_err(|e| format!("wpctl not found: {e}"))?;
    if st.success() {
        Ok(())
    } else {
        Err(format!("wpctl {} failed", args.join(" ")))
    }
}

/// Extract devices from the `Sinks:` / `Sources:` sub-tree of `wpctl status`.
fn section(text: &str, header: &str) -> Vec<AudioDevice> {
    let needle = format!("{header}:");
    let mut out = Vec::new();
    let mut inside = false;

    for line in text.lines() {
        let is_header = (line.contains("├─") || line.contains("└─")) && line.trim_end().ends_with(':');
        if !inside {
            if line.contains(&needle) {
                inside = true;
            }
            continue;
        }
        // Reached the next sub-tree header → done with our section.
        if is_header {
            break;
        }
        if let Some(dev) = parse_line(line) {
            out.push(dev);
        }
    }
    out
}

fn parse_line(line: &str) -> Option<AudioDevice> {
    let is_default = line.contains('*');
    let s = line
        .trim_start_matches(|c: char| {
            c == '│' || c == '├' || c == '└' || c == '─' || c == ' ' || c == '*'
        })
        .trim();

    let (id_part, rest) = s.split_once('.')?;
    let id: u32 = id_part.trim().parse().ok()?;
    let rest = rest.trim();

    let (name, vol_info) = match rest.split_once('[') {
        Some((n, v)) => (n.trim(), v.trim_end_matches(']')),
        None => (rest, ""),
    };
    if name.is_empty() {
        return None;
    }

    let muted = vol_info.contains("MUTED");
    let volume = vol_info
        .split_whitespace()
        .find_map(|tok| tok.parse::<f32>().ok())
        .map(|f| (f * 100.0).round() as u32);

    Some(AudioDevice {
        index: id,
        name: id.to_string(), // switching key for wpctl is the node id
        description: name.to_string(),
        state: String::new(),
        muted,
        volume,
        is_default,
        kind: detect_kind(name).to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
Audio
 ├─ Sinks:
 │  *   49. Built-in Audio Analog Stereo  [vol: 0.65]
 │      52. GB206 HDMI Digital Stereo     [vol: 0.40 MUTED]
 ├─ Sources:
 │  *   51. Built-in Audio Analog Stereo  [vol: 1.00]
 ├─ Filters:
";

    #[test]
    fn parses_sinks_section_only() {
        let sinks = section(SAMPLE, "Sinks");
        assert_eq!(sinks.len(), 2);

        let first = &sinks[0];
        assert_eq!(first.name, "49"); // switching key = node id
        assert!(first.is_default);
        assert_eq!(first.volume, Some(65));
        assert!(!first.muted);

        let hdmi = &sinks[1];
        assert_eq!(hdmi.name, "52");
        assert!(!hdmi.is_default);
        assert_eq!(hdmi.volume, Some(40));
        assert!(hdmi.muted);
        assert_eq!(hdmi.kind, "Hdmi");
    }

    #[test]
    fn parses_sources_section() {
        let sources = section(SAMPLE, "Sources");
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].name, "51");
        assert!(sources[0].is_default);
    }
}
